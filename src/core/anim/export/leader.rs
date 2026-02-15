use std::fs;
use std::sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}};
use std::thread;

use crate::core::anim::export::encoding::{self, ExportConfig, ExportFormat, EncoderMessage, EncoderStatus};
use crate::core::addons::toolpaths::{self, Presence};
use crate::core::addons::avifenc::encoding as avif_addon;
use crate::core::addons::ffmpeg::encoding as ffmpeg_addon;

pub fn start_encoding_thread(
    config: ExportConfig, 
    rx: mpsc::Receiver<EncoderMessage>,
    status_tx: mpsc::Sender<EncoderStatus>,
    abort_signal: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        // Setup Directories
        if let Some(parent) = config.output_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        // Determine Temp File Name
        let ext = match config.format {
            ExportFormat::Gif => "gif",
            ExportFormat::WebP => "webp",
            ExportFormat::Avif => "avif",
            ExportFormat::Png => "png",
            ExportFormat::Mp4 => "mp4",
            ExportFormat::Mkv => "mkv",
            ExportFormat::Webm => "webm",
            ExportFormat::Zip => "zip",
        };
        
        let file_stem = config.output_path.file_stem().unwrap_or_default().to_string_lossy();
        let temp_filename = format!("{}.{}.tmp", file_stem, ext);
        let temp_path = config.output_path.with_file_name(temp_filename);
        
        let final_path = config.output_path.clone();
        let final_tx = status_tx.clone();

        // Decision Logic
        let success = match config.format {
            // AVIFENC (AVIF)
            ExportFormat::Avif if toolpaths::avifenc_status() == Presence::Installed => {
                avif_addon::encode(config.clone(), rx, status_tx, &temp_path, abort_signal.clone())
            },

            // FFmpeg (GIF, WebP, PNG, MP4, MKV, WebM)
            ExportFormat::Gif | ExportFormat::WebP | ExportFormat::Png | ExportFormat::Mp4 | ExportFormat::Mkv | ExportFormat::Webm | ExportFormat::Avif
            if toolpaths::ffmpeg_status() == Presence::Installed => {
                ffmpeg_addon::encode(config.clone(), rx, status_tx, &temp_path, abort_signal.clone())
            },
            
            // Native (WebP, GIF, ZIP)
            _ => {
                encoding::encode_native(config.clone(), rx, status_tx, &temp_path, abort_signal.clone())
            }
        };

        // Atomic Rename or Cleanup
        if success && !abort_signal.load(Ordering::Relaxed) {
            if temp_path.exists() {
                if final_path.exists() { let _ = fs::remove_file(&final_path); }
                let _ = fs::rename(temp_path, final_path);
            }
        } else {
            if temp_path.exists() { let _ = fs::remove_file(temp_path); }
        }

        // Tell UI we are done
        let _ = final_tx.send(EncoderStatus::Finished);
    })
}