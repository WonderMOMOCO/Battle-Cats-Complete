use std::fs;
use std::sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}};
use std::thread;

use crate::features::animation::export::encoding::{self, ExportConfig, ExportFormat, EncoderMessage, EncoderStatus};
use crate::features::addons::toolpaths::{self, Presence};
use crate::features::addons::avifenc::encoding as avif_addon;
use crate::features::addons::ffmpeg::encoding as ffmpeg_addon;

pub fn start_encoding_thread(
    config: ExportConfig, 
    receiver: mpsc::Receiver<EncoderMessage>,
    status_sender: mpsc::Sender<EncoderStatus>,
    abort_signal: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        // Setup Directories
        if let Some(parent_directory) = config.output_path.parent() {
            let _ = fs::create_dir_all(parent_directory);
        }

        // Determine Temp File Name
        let file_extension = match config.format {
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
        let temporary_filename = format!("{}.{}.tmp", file_stem, file_extension);
        let temporary_path = config.output_path.with_file_name(temporary_filename);
        
        let final_path = config.output_path.clone();
        let final_sender = status_sender.clone();

        // Decision Logic
        let is_success = match config.format {
            // AVIFENC (AVIF)
            ExportFormat::Avif if toolpaths::avifenc_status() == Presence::Installed => {
                avif_addon::encode(config.clone(), receiver, status_sender, &temporary_path, abort_signal.clone())
            },

            // FFmpeg (GIF, WebP, PNG, MP4, MKV, WebM)
            ExportFormat::Gif | ExportFormat::WebP | ExportFormat::Png | ExportFormat::Mp4 | ExportFormat::Mkv | ExportFormat::Webm | ExportFormat::Avif
            if toolpaths::ffmpeg_status() == Presence::Installed => {
                ffmpeg_addon::encode(config.clone(), receiver, status_sender, &temporary_path, abort_signal.clone())
            },
            
            // Native (WebP, GIF, ZIP)
            _ => {
                encoding::encode_native(config.clone(), receiver, status_sender, &temporary_path, abort_signal.clone())
            }
        };

        // Atomic Rename or Cleanup
        let is_aborted = abort_signal.load(Ordering::Relaxed);
        let should_save_file = is_success && !is_aborted;

        if !should_save_file {
            if temporary_path.exists() { 
                let _ = fs::remove_file(&temporary_path); 
            }
            let _ = final_sender.send(EncoderStatus::Finished);
            return;
        }

        if !temporary_path.exists() {
            let _ = final_sender.send(EncoderStatus::Finished);
            return;
        }

        if final_path.exists() { 
            let _ = fs::remove_file(&final_path); 
        }
        
        let _ = fs::rename(&temporary_path, &final_path);

        // Tell UI we are done
        let _ = final_sender.send(EncoderStatus::Finished);
    })
}