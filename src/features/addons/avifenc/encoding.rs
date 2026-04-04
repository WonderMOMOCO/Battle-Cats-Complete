use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::fs;
use std::thread;
use crate::features::animation::export::encoding::{ExportConfig, EncoderMessage, EncoderStatus, prepare_image};
use super::download; 
use crate::features::addons::ffmpeg::download as ffmpeg_dl; 
use crate::features::addons::toolpaths::{self, Presence};

pub fn encode(
    config: ExportConfig, 
    receiver: mpsc::Receiver<EncoderMessage>, 
    status_sender: mpsc::Sender<EncoderStatus>, 
    temp_path: &PathBuf, 
    abort_signal: Arc<AtomicBool>
) -> bool {
    if toolpaths::ffmpeg_status() == Presence::Installed {
        encode_via_pipe(config, receiver, status_sender, temp_path, abort_signal)
    } else {
        encode_via_folder(config, receiver, status_sender, temp_path, abort_signal)
    }
}

// FFmpeg -> Pipe -> Avifenc
fn encode_via_pipe(
    config: ExportConfig, 
    receiver: mpsc::Receiver<EncoderMessage>, 
    status_sender: mpsc::Sender<EncoderStatus>, 
    temp_path: &PathBuf, 
    abort_signal: Arc<AtomicBool>
) -> bool {
    let Some(avif_path) = download::get_avif_path() else { return false; };
    let Some(ffmpeg_path) = ffmpeg_dl::get_ffmpeg_path() else { return false; };

    let output_path_string = temp_path.to_string_lossy();
    
    // SPEED
    let speed_value = 10 - (config.compression_percent / 10).clamp(0, 10);
    
    // QUALITY
    let quality_value = ((config.quality_percent as f32 / 100.0) * 63.0).round() as u8;

    // Start Avifenc
    let mut avif_command_builder = Command::new(avif_path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        avif_command_builder.creation_flags(0x08000000);
    }
    
    // Construct Args
    let arguments = vec![
        "--speed".to_string(), speed_value.to_string(),
        "-o".to_string(), output_path_string.to_string(),
        "-q".to_string(), quality_value.to_string(),
        "--qalpha".to_string(), quality_value.to_string(),
        "--stdin".to_string()
    ];

    let Ok(mut avif_command) = avif_command_builder.args(&arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn() else { return false; };

    let Some(mut avif_stdin) = avif_command.stdin.take() else {
        let _ = avif_command.kill();
        return false;
    };

    // Start FFmpeg
    let mut ffmpeg_command_builder = Command::new(ffmpeg_path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        ffmpeg_command_builder.creation_flags(0x08000000);
    }
    
    let Ok(mut ffmpeg_command) = ffmpeg_command_builder.args(&[
        "-f", "rawvideo", 
        "-pixel_format", "rgba", 
        "-video_size", &format!("{}x{}", config.width, config.height), 
        "-framerate", &config.fps.to_string(), 
        "-i", "-", 
        "-f", "yuv4mpegpipe", 
        "-strict", "-1", 
        "-pix_fmt", "yuva444p", "-"
    ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped()) 
        .stderr(Stdio::null())
        .spawn() else {
            let _ = avif_command.kill();
            return false;
        };

    let Some(mut ffmpeg_stdin) = ffmpeg_command.stdin.take() else {
        let _ = avif_command.kill();
        let _ = ffmpeg_command.kill();
        return false;
    };
    
    let Some(mut ffmpeg_stdout) = ffmpeg_command.stdout.take() else {
        let _ = avif_command.kill();
        let _ = ffmpeg_command.kill();
        return false;
    };

    // Process Decoupling Bridge
    let bridge_handle = thread::spawn(move || {
        let _ = std::io::copy(&mut ffmpeg_stdout, &mut avif_stdin);
    });

    let mut frames_processed = 0;
    let mut is_success = false;

    // Pump frames to FFmpeg
    while let Ok(message) = receiver.recv() {
        if abort_signal.load(Ordering::Relaxed) { break; }

        match message {
            EncoderMessage::Frame(raw_pixels, w, h, _) => {
                if status_sender.send(EncoderStatus::Progress(frames_processed)).is_err() { break; }
                let image_data = prepare_image(raw_pixels, w, h, config.background);
                if ffmpeg_stdin.write_all(&image_data.into_vec()).is_err() { break; }
                frames_processed += 1;
            },
            EncoderMessage::Finish => { is_success = true; break; }
        }
    }

    drop(ffmpeg_stdin); 

    if !is_success || abort_signal.load(Ordering::Relaxed) {
        let _ = ffmpeg_command.kill();
        let _ = avif_command.kill();
        return false;
    }

    let _ = bridge_handle.join();
    let _ = ffmpeg_command.wait();
    let avif_status = avif_command.wait();

    is_success && avif_status.map(|status| status.success()).unwrap_or(false)
}

// Raw Frames -> Folder -> Avifenc
fn encode_via_folder(
    config: ExportConfig, 
    receiver: mpsc::Receiver<EncoderMessage>, 
    status_sender: mpsc::Sender<EncoderStatus>, 
    temp_path: &PathBuf, 
    abort_signal: Arc<AtomicBool>
) -> bool {
    let Some(avifenc_path) = download::get_avif_path() else { return false; };
    let folder_name = format!("{}.temp", temp_path.file_stem().unwrap_or_default().to_string_lossy());
    
    let parent_directory = temp_path.parent().unwrap_or_else(|| Path::new("."));
    let work_directory = parent_directory.join(folder_name);
    
    // Ensure we start clean
    if work_directory.exists() { let _ = fs::remove_dir_all(&work_directory); }
    let _ = fs::create_dir_all(&work_directory);

    let mut frames_processed = 0;
    let mut frame_paths = Vec::new();

    // Pump frames to PNGs
    while let Ok(message) = receiver.recv() {
        if abort_signal.load(Ordering::Relaxed) { 
            let _ = fs::remove_dir_all(&work_directory);
            return false; 
        }
        match message {
            EncoderMessage::Frame(raw_pixels, w, h, _) => {
                let image_data = prepare_image(raw_pixels, w, h, config.background);
                let current_frame_path = work_directory.join(format!("frame_{:05}.png", frames_processed));
                if image_data.save(&current_frame_path).is_ok() {
                    frame_paths.push(current_frame_path);
                    frames_processed += 1;
                    let _ = status_sender.send(EncoderStatus::Progress(frames_processed));
                }
            },
            EncoderMessage::Finish => break,
        }
    }

    if frame_paths.is_empty() { 
        let _ = fs::remove_dir_all(&work_directory);
        return false; 
    }

    // MAPPING
    let speed_value = 10 - (config.compression_percent / 10).clamp(0, 10);
    let quality_value = ((config.quality_percent as f32 / 100.0) * 63.0).round() as u8;

    let mut arguments = vec![
        "--speed".to_string(), speed_value.to_string(),
        "-o".to_string(), temp_path.to_string_lossy().to_string(),
        "-q".to_string(), quality_value.to_string()
    ];

    for frame_path in &frame_paths { 
        arguments.push(frame_path.to_string_lossy().to_string()); 
    }

    let mut avif_command_builder = Command::new(avifenc_path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        avif_command_builder.creation_flags(0x08000000);
    }
    
    let Ok(mut child_process) = avif_command_builder.args(&arguments)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn() else {
            let _ = fs::remove_dir_all(&work_directory);
            return false;
        };

    let mut is_finished = false;
    let mut is_success = false;
    
    while !is_finished {
        if abort_signal.load(Ordering::Relaxed) {
            let _ = child_process.kill();
            let _ = child_process.wait();
            let _ = fs::remove_dir_all(&work_directory);
            return false;
        }

        match child_process.try_wait() {
            Ok(Some(status)) => {
                is_finished = true;
                is_success = status.success();
            },
            Ok(None) => {
                thread::sleep(std::time::Duration::from_millis(50));
            },
            Err(_) => {
                let _ = child_process.kill();
                is_finished = true;
                is_success = false;
            }
        }
    }

    let _ = fs::remove_dir_all(&work_directory);
    is_success
}