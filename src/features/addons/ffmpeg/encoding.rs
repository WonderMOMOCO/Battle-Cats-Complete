use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}};
use std::path::PathBuf;
use std::thread;
use std::io::Write;
use crate::features::animation::export::encoding::{ExportConfig, ExportFormat, EncoderMessage, EncoderStatus, prepare_image};
use super::download;

pub fn encode(
    config: ExportConfig, 
    receiver: mpsc::Receiver<EncoderMessage>, 
    status_sender: mpsc::Sender<EncoderStatus>, 
    temp_path: &PathBuf, 
    abort_signal: Arc<AtomicBool>
) -> bool {
    let Some(ffmpeg_path) = download::get_ffmpeg_path() else { return false; };

    // BUILD ARGUMENTS BASED ON FORMAT
    let mut arguments = vec![
        "-nostdin".to_string(),
        "-f".to_string(), "rawvideo".to_string(),
        "-pixel_format".to_string(), "rgba".to_string(),
        "-video_size".to_string(), format!("{}x{}", config.width, config.height),
        "-framerate".to_string(), config.fps.to_string(),
        "-i".to_string(), "-".to_string(),
    ];

    match config.format {
        ExportFormat::Gif => {
            let dither = if config.quality_percent >= 80 { "sierra2_4a" } 
                         else if config.quality_percent >= 40 { "floyd_steinberg" } 
                         else { "bayer:bayer_scale=5" };
            let stats_mode = if config.compression_percent < 50 { "full" } else { "diff" };
            let filter = format!("split[s0][s1];[s0]palettegen=stats_mode={}[p];[s1][p]paletteuse=dither={}", stats_mode, dither);

            arguments.extend_from_slice(&[
                "-vf".to_string(), filter,
                "-f".to_string(), "gif".to_string(),
            ]);
        },
        ExportFormat::WebP => {
            let compression_level = (config.compression_percent as f32 / 100.0 * 6.0).round() as u8;
            arguments.extend_from_slice(&[
                "-c:v".to_string(), "libwebp_anim".to_string(),
                "-loop".to_string(), "0".to_string(),
                "-q:v".to_string(), config.quality_percent.to_string(), 
                "-compression_level".to_string(), compression_level.to_string(), 
                "-preset".to_string(), "drawing".to_string(),
                "-threads".to_string(), "0".to_string(),
                "-f".to_string(), "webp".to_string(),
            ]);
        },
        ExportFormat::Png => {
            arguments.extend_from_slice(&[
                "-plays".to_string(), "0".to_string(),
                "-c:v".to_string(), "apng".to_string(),
                "-f".to_string(), "apng".to_string(),
            ]);
        },
        ExportFormat::Mp4 | ExportFormat::Mkv | ExportFormat::Webm => {
            let use_av1 = config.quality_percent > 90 && config.compression_percent > 90;
            let needs_even_dims = use_av1 || config.format != ExportFormat::Webm;

            if needs_even_dims {
                arguments.extend_from_slice(&["-vf".to_string(), "crop=trunc(iw/2)*2:trunc(ih/2)*2".to_string()]);
            }

            // AV1 LOGIC
            if use_av1 {
                let crf_value = 63.0 - (config.quality_percent as f32 / 100.0 * 63.0); 
                let cpu_used_value = 4.0 + (config.compression_percent as f32 / 100.0 * 4.0);
                
                arguments.extend_from_slice(&[
                    "-c:v".to_string(), "libaom-av1".to_string(),
                    "-pix_fmt".to_string(), "yuv420p".to_string(), // Strict
                    "-crf".to_string(), format!("{:.0}", crf_value),
                    "-cpu-used".to_string(), format!("{:.0}", cpu_used_value),
                    "-b:v".to_string(), "0".to_string(), // Constant quality mode
                    "-strict".to_string(), "experimental".to_string(),
                ]);
            } else {
                // Standard Codecs
                match config.format {
                    ExportFormat::Webm => {
                        // VP9
                        let crf_value = 63.0 - (config.quality_percent as f32 / 100.0 * 63.0);
                        arguments.extend_from_slice(&[
                            "-c:v".to_string(), "libvpx-vp9".to_string(),
                            "-pix_fmt".to_string(), "yuva420p".to_string(), 
                            "-crf".to_string(), format!("{:.0}", crf_value),
                            "-b:v".to_string(), "0".to_string(),
                        ]);
                    },
                    _ => {
                        // H.264
                        let crf_value = 51.0 - (config.quality_percent as f32 / 100.0 * 33.0); 
                        let presets = ["ultrafast", "superfast", "veryfast", "faster", "fast", "medium", "slow", "slower", "veryslow"];
                        let preset_index = (config.compression_percent as f32 / 100.0 * 8.0).round() as usize;
                        
                        arguments.extend_from_slice(&[
                            "-c:v".to_string(), "libx264".to_string(),
                            "-pix_fmt".to_string(), "yuv420p".to_string(),
                            "-profile:v".to_string(), "main".to_string(),
                            "-crf".to_string(), format!("{:.0}", crf_value),
                            "-preset".to_string(), presets[preset_index].to_string(),
                        ]);
                    }
                }
            }

            // Container Format
            let container_format = match config.format {
                ExportFormat::Mp4 => "mp4",
                ExportFormat::Mkv => "matroska",
                ExportFormat::Webm => "webm",
                _ => "mp4",
            };
            arguments.extend_from_slice(&["-f".to_string(), container_format.to_string()]);
        },
        _ => return false,
    }

    // Output path and overwrite flag
    arguments.push("-y".to_string());
    arguments.push(temp_path.to_string_lossy().to_string());

    let mut command_builder = Command::new(ffmpeg_path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command_builder.creation_flags(0x08000000);
    }
    
    let Ok(mut child_process) = command_builder.args(&arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn() else { return false; };

    let Some(mut ffmpeg_stdin) = child_process.stdin.take() else {
        let _ = child_process.kill();
        return false;
    };
    
    let progress_sender = status_sender.clone();
    let abort_signal_clone = abort_signal.clone();
    
    // Pump Thread
    let input_handle = thread::spawn(move || {
        let mut frames_processed = 0;
        let mut finished_cleanly = false;
        
        while let Ok(message) = receiver.recv() {
            if abort_signal_clone.load(Ordering::Relaxed) { break; }

            match message {
                EncoderMessage::Frame(raw_pixels, w, h, _) => {
                    if progress_sender.send(EncoderStatus::Progress(frames_processed)).is_err() { break; } 
                    
                    let image_data = prepare_image(raw_pixels, w, h, config.background);
                    
                    if ffmpeg_stdin.write_all(&image_data.into_vec()).is_err() { break; }
                    frames_processed += 1;
                },
                EncoderMessage::Finish => { finished_cleanly = true; break; }
            }
        }
        drop(ffmpeg_stdin);
        finished_cleanly
    });

    let did_input_succeed = input_handle.join().unwrap_or(false);
    
    // KILL IF ABORTED OR FAILED
    if abort_signal.load(Ordering::Relaxed) || !did_input_succeed {
        let _ = child_process.kill();
        let _ = child_process.wait();
        return false;
    }

    let did_process_succeed = child_process.wait().map(|status| status.success()).unwrap_or(false);
    did_process_succeed
}