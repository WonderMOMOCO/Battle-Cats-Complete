use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}};
use std::io::Write;
use std::path::PathBuf;
use std::fs;
use std::thread;
use crate::core::anim::export::encoding::{ExportConfig, EncoderMessage, EncoderStatus, prepare_image};
use super::download; 
use crate::core::addons::ffmpeg::download as ffmpeg_dl; 
use crate::core::addons::toolpaths::{self, Presence};

pub fn encode(
    config: ExportConfig, 
    rx: mpsc::Receiver<EncoderMessage>, 
    status_tx: mpsc::Sender<EncoderStatus>, 
    temp_path: &PathBuf, 
    abort_signal: Arc<AtomicBool>
) -> bool {
    if toolpaths::ffmpeg_status() == Presence::Installed {
        encode_via_pipe(config, rx, status_tx, temp_path, abort_signal)
    } else {
        encode_via_folder(config, rx, status_tx, temp_path, abort_signal)
    }
}

// FFmpeg -> Pipe -> Avifenc
fn encode_via_pipe(
    config: ExportConfig, 
    rx: mpsc::Receiver<EncoderMessage>, 
    status_tx: mpsc::Sender<EncoderStatus>, 
    temp_path: &PathBuf, 
    abort_signal: Arc<AtomicBool>
) -> bool {
    let avif_path = match download::get_avif_path() { Some(p) => p, None => return false };
    let ffmpeg_path = match ffmpeg_dl::get_ffmpeg_path() { Some(p) => p, None => return false };

    let out_path_str = temp_path.to_string_lossy();
    
    // Start Avifenc
    let mut cmd = Command::new(avif_path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut avif_cmd = cmd.args(&["--stdin", "--speed", "8", "-q", "60", "--qalpha", "60", "-o", &out_path_str])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Avifenc Fail");

    let mut avif_stdin = avif_cmd.stdin.take().expect("Stdin Fail");

    // Start FFmpeg
    let mut cmd = Command::new(ffmpeg_path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut ffmpeg_cmd = cmd.args(&["-f", "rawvideo", "-pixel_format", "rgba", "-video_size", &format!("{}x{}", config.width, config.height), "-framerate", &config.fps.to_string(), "-i", "-", "-f", "yuv4mpegpipe", "-strict", "-1", "-pix_fmt", "yuva444p", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped()) 
        .stderr(Stdio::null())
        .spawn()
        .expect("FFmpeg Fail");

    let mut ff_stdin = ffmpeg_cmd.stdin.take().expect("FF Stdin Fail");
    let mut ff_stdout = ffmpeg_cmd.stdout.take().expect("FF Stdout Fail");

    // Process Decoupling Bridge
    // This thread ensures FFmpeg can flush its buffer even if Avifenc is busy
    let bridge_handle = thread::spawn(move || {
        let _ = std::io::copy(&mut ff_stdout, &mut avif_stdin);
    });

    let mut frames = 0;
    let mut success = false;

    // Pump frames to FFmpeg
    while let Ok(msg) = rx.recv() {
        if abort_signal.load(Ordering::Relaxed) { break; }

        match msg {
            EncoderMessage::Frame(raw_pixels, w, h, _) => {
                if status_tx.send(EncoderStatus::Progress(frames)).is_err() { break; }
                let img = prepare_image(raw_pixels, w, h);
                if ff_stdin.write_all(&img.into_vec()).is_err() { break; }
                frames += 1;
            },
            EncoderMessage::Finish => { success = true; break; }
        }
    }

    drop(ff_stdin); 

    if !success || abort_signal.load(Ordering::Relaxed) {
        let _ = ffmpeg_cmd.kill();
        let _ = avif_cmd.kill();
        return false;
    }

    // Wait for the data to finish flowing through the bridge
    let _ = bridge_handle.join();
    
    let _ = ffmpeg_cmd.wait();
    let avif_status = avif_cmd.wait();

    success && avif_status.map(|s| s.success()).unwrap_or(false)
}

// Raw Frames -> Folder -> Avifenc
fn encode_via_folder(
    _config: ExportConfig, 
    rx: mpsc::Receiver<EncoderMessage>, 
    status_tx: mpsc::Sender<EncoderStatus>, 
    temp_path: &PathBuf, 
    abort: Arc<AtomicBool>
) -> bool {
    let avifenc_path = match download::get_avif_path() { Some(p) => p, None => return false };
    let folder_name = format!("{}.temp", temp_path.file_stem().unwrap_or_default().to_string_lossy());
    let work_dir = temp_path.parent().unwrap_or(&PathBuf::from(".")).join(folder_name);
    
    // Ensure we start clean
    if work_dir.exists() { let _ = fs::remove_dir_all(&work_dir); }
    let _ = fs::create_dir_all(&work_dir);

    let mut frames_processed = 0;
    let mut frame_paths = Vec::new();

    // Pump frames to PNGs
    while let Ok(msg) = rx.recv() {
        if abort.load(Ordering::Relaxed) { 
            let _ = fs::remove_dir_all(&work_dir);
            return false; 
        }
        match msg {
            EncoderMessage::Frame(raw_pixels, w, h, _) => {
                let img = prepare_image(raw_pixels, w, h);
                let p = work_dir.join(format!("frame_{:05}.png", frames_processed));
                if img.save(&p).is_ok() {
                    frame_paths.push(p);
                    frames_processed += 1;
                    let _ = status_tx.send(EncoderStatus::Progress(frames_processed));
                }
            },
            EncoderMessage::Finish => break,
        }
    }

    if frame_paths.is_empty() { 
        let _ = fs::remove_dir_all(&work_dir);
        return false; 
    }

    // Run Avifenc on the folder
    let mut args = vec![
        "--speed".to_string(), "8".to_string(),
        "-q".to_string(), "60".to_string(),
        "-o".to_string(), temp_path.to_string_lossy().to_string(),
    ];
    for p in &frame_paths { args.push(p.to_string_lossy().to_string()); }

    let mut cmd = Command::new(avifenc_path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut child = cmd.args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Avifenc Fallback Fail");

    // Monitor Process (Polling for Abort)
    let mut finished = false;
    let mut success = false;
    
    while !finished {
        if abort.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = fs::remove_dir_all(&work_dir); // WIPE FOLDER
            return false;
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                finished = true;
                success = status.success();
            },
            Ok(None) => {
                thread::sleep(std::time::Duration::from_millis(50));
            },
            Err(_) => {
                let _ = child.kill();
                finished = true;
                success = false;
            }
        }
    }

    // Cleanup
    let _ = fs::remove_dir_all(&work_dir);
    success
}