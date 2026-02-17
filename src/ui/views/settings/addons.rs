use eframe::egui;
use crate::core::settings::Settings;
use crate::core::addons::adb::download::AdbManager;
use crate::core::addons::avifenc::download::AvifManager;
use crate::core::addons::ffmpeg::download::FfmpegManager;
#[cfg(target_os = "windows")]
use crate::core::addons::oem::download::{OemManager, OemDriver}; 
use crate::core::addons::toolpaths::AddonStatus;
use crate::core::utils::DragGuard;
use std::sync::Mutex;

#[derive(Default, Clone)]
pub struct AddonDeleteState {
    pub is_open: bool,
    pub target_name: String,
}

static ADB_MANAGER: Mutex<Option<AdbManager>> = Mutex::new(None);
static AVIF_MANAGER: Mutex<Option<AvifManager>> = Mutex::new(None);
static FFMPEG_MANAGER: Mutex<Option<FfmpegManager>> = Mutex::new(None);

#[cfg(target_os = "windows")]
static OEM_MANAGER: Mutex<Option<OemManager>> = Mutex::new(None);

pub fn show(ui: &mut egui::Ui, _settings: &mut Settings, drag_guard: &mut DragGuard) -> bool {
    {
        let mut adb_lock = ADB_MANAGER.lock().unwrap();
        let adb_manager = adb_lock.get_or_insert_with(AdbManager::default);
        adb_manager.update();

        #[cfg(target_os = "windows")]
        let mut oem_lock = OEM_MANAGER.lock().unwrap();
        #[cfg(target_os = "windows")]
        let oem_manager = oem_lock.get_or_insert_with(OemManager::default);

        let mut avif_lock = AVIF_MANAGER.lock().unwrap();
        let avif_manager = avif_lock.get_or_insert_with(AvifManager::default);
        avif_manager.update();

        let mut ffmpeg_lock = FFMPEG_MANAGER.lock().unwrap();
        let ffmpeg_manager = ffmpeg_lock.get_or_insert_with(FfmpegManager::default);
        ffmpeg_manager.update();

        egui::ScrollArea::vertical()
            .id_salt("addons_scroll")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.heading("Android Bridge");
                ui.add_space(5.0);
                ui.label("Enables \"Android\" option for Game Data Import\nRequired for connecting to phones\nBase Add-On supports emulators only");
                ui.add_space(8.0);
                
                let adb_status = adb_manager.status.clone(); 
                render_addon_controls(ui, &adb_status, "ADB", || adb_manager.install(), "adb_delete");

                #[cfg(target_os = "windows")]
                {
                    ui.add_space(20.0);
                    ui.heading(egui::RichText::new("ADB OEM Drivers").strong());
                    ui.label("Allows \"Android\" export to connect to a real Android device for game files\nWindows only, requires Android Bridge Add-On, and manual set-up\nRequires USB Debugging to be enabled on your Android device");
                    
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("oem_combo")
                            .selected_text(OemManager::label(oem_manager.selected)) 
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                for driver in OemManager::all_drivers() {
                                    ui.selectable_value(
                                        &mut oem_manager.selected, 
                                        driver, 
                                        OemManager::label(driver) 
                                    );
                                }
                            });

                        let btn_text = if oem_manager.selected == OemDriver::Universal {
                            "Download Installer"
                        } else {
                            "Open Download Page"
                        };

                        if ui.button(btn_text).clicked() {
                            oem_manager.execute_action();
                        }
                    });
                }

                ui.add_space(20.0);
                ui.heading("FFMPEG");
                ui.add_space(5.0);
                ui.label("Optimizes encoding speed for most file formats\nEnables most export formats");
                ui.add_space(8.0);
                let ffmpeg_status = ffmpeg_manager.status.clone();
                render_addon_controls(ui, &ffmpeg_status, "FFMPEG", || ffmpeg_manager.install(), "ffmpeg_delete");

                ui.add_space(20.0);
                ui.heading("AVIFENC");
                ui.add_space(5.0);
                ui.label("Optimizes encoding for the AVIF format specifically\nEnables AVIF export format");
                ui.add_space(8.0);
                let avif_status = avif_manager.status.clone();
                render_addon_controls(ui, &avif_status, "AVIFENC", || avif_manager.install(), "avif_delete");
            });
    }

    handle_modals(ui.ctx(), drag_guard);

    false
}

fn handle_modals(ctx: &egui::Context, drag_guard: &mut DragGuard) {
    let mut adb_lock = ADB_MANAGER.lock().unwrap();
    let adb_manager = adb_lock.get_or_insert_with(AdbManager::default);

    let mut avif_lock = AVIF_MANAGER.lock().unwrap();
    let avif_manager = avif_lock.get_or_insert_with(AvifManager::default);

    let mut ffmpeg_lock = FFMPEG_MANAGER.lock().unwrap();
    let ffmpeg_manager = ffmpeg_lock.get_or_insert_with(FfmpegManager::default);

    handle_delete_modal(ctx, drag_guard, "adb_delete", || adb_manager.uninstall());
    handle_delete_modal(ctx, drag_guard, "avif_delete", || avif_manager.uninstall());
    handle_delete_modal(ctx, drag_guard, "ffmpeg_delete", || ffmpeg_manager.uninstall());
}

fn render_addon_controls(ui: &mut egui::Ui, status: &AddonStatus, name: &str, on_download: impl FnOnce(), confirm_id: &str) {
    match status {
        AddonStatus::Installed => {
            let btn = egui::Button::new(format!("Delete {}", name)).fill(egui::Color32::from_rgb(180, 50, 50));
            if ui.add_sized([140.0, 30.0], btn).clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(confirm_id), AddonDeleteState { 
                    is_open: true, 
                    target_name: name.to_string() 
                }));
            }
        },
        AddonStatus::Downloading(_, _) => {
            let btn = egui::Button::new(format!("Downloading {}", name))
                .fill(egui::Color32::from_rgb(200, 180, 50));
            ui.add_sized([140.0, 30.0], btn);
            ui.ctx().request_repaint();
        },
        AddonStatus::NotInstalled | AddonStatus::Error(_) => {
            let btn = egui::Button::new(format!("Download {}", name)).fill(egui::Color32::from_rgb(40, 160, 40));
            if ui.add_sized([140.0, 30.0], btn).clicked() {
                on_download();
            }
            if let AddonStatus::Error(e) = status {
                ui.add_space(5.0);
                ui.label(egui::RichText::new(format!("Error: {}", e)).color(egui::Color32::RED));
            }
        },
    }
}

fn handle_delete_modal(ctx: &egui::Context, drag_guard: &mut DragGuard, id: &str, on_yes: impl FnOnce()) {
    let state_id = egui::Id::new(id);
    let mut state = ctx.data(|d| d.get_temp::<AddonDeleteState>(state_id)).unwrap_or_default();

    if state.is_open {
        let allow_drag = drag_guard.update(ctx);
        let mut should_close = false;

        egui::Window::new("Confirm Deletion")
            .id(egui::Id::new(format!("{}_window", id)))
            .collapsible(false)
            .resizable(false)
            .movable(allow_drag) 
            .default_pos(ctx.screen_rect().center())
            .pivot(egui::Align2::CENTER_CENTER)
            .show(ctx, |ui| {
                ui.set_min_width(220.0);
                ui.vertical_centered(|ui| {
                    ui.add_space(5.0);
                    ui.label(format!("Are you sure you want to delete {}?", state.target_name)); 
                    ui.add_space(15.0);
                    
                    ui.horizontal(|ui| {
                        let total_width = 130.0; // [60.0] + [60.0] + 10.0 spacing
                        let x_offset = (ui.available_width() - total_width) / 2.0;
                        ui.add_space(x_offset);

                        if ui.add_sized([60.0, 30.0], egui::Button::new("Yes")).clicked() {
                            on_yes();
                            should_close = true;
                        }
                        
                        ui.add_space(10.0);

                        if ui.add_sized([60.0, 30.0], egui::Button::new("No")).clicked() {
                            should_close = true;
                        }
                    });
                    ui.add_space(5.0);
                });
            });
        
        if should_close {
            state.is_open = false;
        }
        
        ctx.data_mut(|d| d.insert_temp(state_id, state));
    }
}