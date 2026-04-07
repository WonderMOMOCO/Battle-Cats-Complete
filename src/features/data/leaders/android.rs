use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use crate::features::data::utilities::engine;
use crate::features::data::state::{AdbImportType, AdbRegion};
use crate::features::addons::adb::bridge;
use crate::features::settings::logic::state::EmulatorConfig;

pub fn run(
    status_sender: Sender<String>, 
    import_mode: AdbImportType, 
    target_region: AdbRegion, 
    emulator_config: EmulatorConfig,
    abort_flag: Arc<AtomicBool>,
    job_status: Arc<AtomicU8>,
    progress_current: Arc<AtomicUsize>,
    progress_maximum: Arc<AtomicUsize>
) {
    thread::spawn(move || {
        let terminate_job = |status_tracker: Arc<AtomicU8>, is_error: bool| { 
            status_tracker.store(if is_error { 3 } else { 2 }, Ordering::Relaxed); 
        };
        
        let app_repository_directory = PathBuf::from("game/app");

        let pull_result = bridge::execute_pull(
            &app_repository_directory, 
            import_mode, 
            target_region, 
            &emulator_config, 
            &status_sender, 
            &abort_flag
        );

        if abort_flag.load(Ordering::Relaxed) { 
            return terminate_job(job_status, true); 
        }

        match pull_result {
            Ok(pulled_package_directories) => {
                let _ = status_sender.send("Starting Processing Phase...".to_string());
                
                if let Err(engine_error) = engine::run_universal_import(&pulled_package_directories, &status_sender, &abort_flag, &progress_current, &progress_maximum) {
                    let _ = status_sender.send(format!("Universal Import Failed: {}", engine_error));
                    return terminate_job(job_status, true);
                }

                if !emulator_config.keep_app_folder {
                    let _ = status_sender.send("Cleaning up app package files...".to_string());
                    for package_directory in pulled_package_directories {
                        let _ = std::fs::remove_dir_all(package_directory);
                    }
                }
            },
            Err(bridge_error) => {
                let _ = status_sender.send(format!("ADB Pull Failed: {}", bridge_error));
                return terminate_job(job_status, true);
            }
        }

        let _ = status_sender.send("All Operations Complete!".to_string());
        terminate_job(job_status, false);
    });
}