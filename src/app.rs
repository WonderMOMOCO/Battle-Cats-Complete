use eframe::egui;

// Global Utilities & Formats
use crate::global::game::param::Param;
use crate::global::io::{json, watcher::GlobalWatcher};
use crate::global::ui::shared::DragGuard;

// Feature States
use crate::updater::Updater;
use crate::features::data::logic::ImportState;
use crate::features::cat::logic::CatListState;
use crate::features::enemy::logic::state::EnemyListState;
use crate::features::stage::logic::state::StageListState;
use crate::features::mods::logic::state::ModState;
use crate::features::settings::logic::Settings;

pub mod startup;
pub mod frame;
pub mod reload;
pub mod events;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] 
pub struct BattleCatsApp {
    #[serde(skip)] pub(crate) current_page: frame::Page,
    #[serde(skip)] pub(crate) sidebar_open: bool,
    #[serde(skip)] pub(crate) import_state: ImportState,
    #[serde(skip)] pub(crate) updater: Updater,
    #[serde(skip)] pub(crate) drag_guard: DragGuard,
    #[serde(skip)] pub(crate) global_watcher: Option<GlobalWatcher>,
    #[serde(skip)] pub param: Param,
    pub(crate) cat_list_state: CatListState,
    pub(crate) enemy_list_state: EnemyListState,
    pub(crate) stage_list_state: StageListState,
    pub(crate) mod_state: ModState,
    pub settings: Settings,
}

impl Default for BattleCatsApp {
    fn default() -> Self {
        Self {
            current_page: frame::Page::Home,
            sidebar_open: false,
            import_state: ImportState::default(),
            cat_list_state: CatListState::default(),
            enemy_list_state: EnemyListState::default(),
            stage_list_state: StageListState::default(),
            mod_state: ModState::default(),
            settings: Settings::default(),
            updater: Updater::default(),
            drag_guard: DragGuard::default(),
            global_watcher: None,
            param: Param::default(),
        }
    }
}

impl eframe::App for BattleCatsApp {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        json::save("settings.json", self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.updater.update_state(ctx);
        
        let status_str = match self.updater.status {
            crate::updater::UpdateStatus::Checking => "Checking",
            crate::updater::UpdateStatus::UpToDate => "UpToDate",
            crate::updater::UpdateStatus::UpdateFound(..) => "UpdateFound",
            crate::updater::UpdateStatus::CheckFailed => "CheckFailed",
            crate::updater::UpdateStatus::Downloading(_) => "Downloading",
            crate::updater::UpdateStatus::RestartPending(_) => "RestartPending",
            crate::updater::UpdateStatus::Idle => "Idle",
        };
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("updater_status"), status_str));

        if self.settings.runtime.manual_check_requested {
            self.settings.runtime.manual_check_requested = false;
            self.updater.check_for_updates(ctx.clone(), true);
        }

        self.updater.show_ui(ctx, &mut self.settings, &mut self.drag_guard);
        
        self.process_file_events(ctx);

        self.cat_list_state.update_data();
        self.enemy_list_state.update_data();
        self.stage_list_state.update_data();

        if self.cat_list_state.scan_receiver.is_some() || self.enemy_list_state.scan_receiver.is_some() || self.stage_list_state.scan_receiver.is_some() {
            ctx.request_repaint();
        }
        
        let import_finished = self.import_state.update(ctx);
        if import_finished {
            self.perform_full_data_reload();
            ctx.request_repaint();
        }

        frame::draw(self, ctx);
    }
}