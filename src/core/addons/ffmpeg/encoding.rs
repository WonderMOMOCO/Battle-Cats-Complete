use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}};
use std::path::PathBuf;
use std::thread;
use std::io::Write;
use crate::core::anim::export::encoding::{ExportConfig, ExportFormat, EncoderMessage, EncoderStatus, prepare_image};
use super::download;

pub fn encode(
    config: ExportConfig, 
    rx: mpsc::Receiver<EncoderMessage>, 
    status_tx: mpsc::Sender<EncoderStatus>, 
    temp_path: &PathBuf, 
    abort_signal: Arc<AtomicBool>
) -> bool {
    let ffmpeg_path = match download::get_ffmpeg_path() { Some(p) => p, None => return false };

    // BUILD ARGUMENTS BASED ON FORMAT
    let mut args = vec![
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

            args.extend_from_slice(&[
                "-vf".to_string(), filter,
                "-f".to_string(), "gif".to_string(),
            ]);
        },
        ExportFormat::WebP => {
            let level = (config.compression_percent as f32 / 100.0 * 6.0).round() as u8;
            args.extend_from_slice(&[
                "-c:v".to_string(), "libwebp_anim".to_string(),
                "-loop".to_string(), "0".to_string(),
                "-q:v".to_string(), config.quality_percent.to_string(), 
                "-compression_level".to_string(), level.to_string(), 
                "-preset".to_string(), "drawing".to_string(),
                "-threads".to_string(), "0".to_string(),
                "-f".to_string(), "webp".to_string(),
            ]);
        },
        ExportFormat::Png => {
            args.extend_from_slice(&[
                "-plays".to_string(), "0".to_string(),
                "-c:v".to_string(), "apng".to_string(),
                "-f".to_string(), "apng".to_string(),
            ]);
        },
        ExportFormat::Mp4 | ExportFormat::Mkv | ExportFormat::Webm => {
            let use_av1 = config.quality_percent > 90 && config.compression_percent > 90;
            let needs_even_dims = use_av1 || config.format != ExportFormat::Webm;

            if needs_even_dims {
                args.extend_from_slice(&["-vf".to_string(), "crop=trunc(iw/2)*2:trunc(ih/2)*2".to_string()]);
            }

            // AV1 LOGIC
            if use_av1 {
                let crf = 63.0 - (config.quality_percent as f32 / 100.0 * 63.0); 
                let cpu_used = 4.0 + (config.compression_percent as f32 / 100.0 * 4.0);
                
                args.extend_from_slice(&[
                    "-c:v".to_string(), "libaom-av1".to_string(),
                    "-pix_fmt".to_string(), "yuv420p".to_string(), // Strict
                    "-crf".to_string(), format!("{:.0}", crf),
                    "-cpu-used".to_string(), format!("{:.0}", cpu_used),
                    "-b:v".to_string(), "0".to_string(), // Constant quality mode
                    "-strict".to_string(), "experimental".to_string(),
                ]);
            } else {
                // Standard Codecs
                match config.format {
                    ExportFormat::Webm => {
                        // VP9
                        let crf = 63.0 - (config.quality_percent as f32 / 100.0 * 63.0);
                        args.extend_from_slice(&[
                            "-c:v".to_string(), "libvpx-vp9".to_string(),
                            "-pix_fmt".to_string(), "yuva420p".to_string(), 
                            "-crf".to_string(), format!("{:.0}", crf),
                            "-b:v".to_string(), "0".to_string(),
                        ]);
                    },
                    _ => {
                        // H.264
                        let crf = 51.0 - (config.quality_percent as f32 / 100.0 * 33.0); 
                        let presets = ["ultrafast", "superfast", "veryfast", "faster", "fast", "medium", "slow", "slower", "veryslow"];
                        let p_idx = (config.compression_percent as f32 / 100.0 * 8.0).round() as usize;
                        
                        args.extend_from_slice(&[
                            "-c:v".to_string(), "libx264".to_string(),
                            "-pix_fmt".to_string(), "yuv420p".to_string(),
                            "-profile:v".to_string(), "main".to_string(),
                            "-crf".to_string(), format!("{:.0}", crf),
                            "-preset".to_string(), presets[p_idx].to_string(),
                        ]);
                    }
                }
            }

            // Container Format
            let fmt = match config.format {
                ExportFormat::Mp4 => "mp4",
                ExportFormat::Mkv => "matroska",
                ExportFormat::Webm => "webm",
                _ => "mp4",
            };
            args.extend_from_slice(&["-f".to_string(), fmt.to_string()]);
        },
        _ => return false,
    }

    // Output path and overwrite flag
    args.push("-y".to_string());
    args.push(temp_path.to_string_lossy().to_string());

    let mut cmd = Command::new(ffmpeg_path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    
    let mut child = cmd.args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("FFmpeg Fail");

    let mut stdin = child.stdin.take().expect("Stdin Fail");
    let tx_progress = status_tx.clone();
    let abort_in = abort_signal.clone();
    
    // Pump Thread
    let input_handle = thread::spawn(move || {
        let mut frames = 0;
        let mut clean = false;
        while let Ok(msg) = rx.recv() {
            if abort_in.load(Ordering::Relaxed) { break; }

            match msg {
                EncoderMessage::Frame(raw_pixels, w, h, _) => {
                    if tx_progress.send(EncoderStatus::Progress(frames)).is_err() { break; } 
                    let img = prepare_image(raw_pixels, w, h);
                    if stdin.write_all(&img.into_vec()).is_err() { break; }
                    frames += 1;
                },
                EncoderMessage::Finish => { clean = true; break; }
            }
        }
        drop(stdin);
        clean
    });

    let input_success = input_handle.join().unwrap_or(false);
    
    // KILL IF ABORTED OR FAILED
    if abort_signal.load(Ordering::Relaxed) || !input_success {
        let _ = child.kill();
        let _ = child.wait();
        return false;
    }

    let process_success = child.wait().map(|s| s.success()).unwrap_or(false);
    process_success
}