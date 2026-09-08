use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use iced::futures::channel::mpsc;
use iced::{Size, Task};
use smol::Timer;
use tracing::{debug, error, info, warn};

use kore::common::architecture;
use kore::common::dirs;
use kore::common::io::json;
use kore::domains::import;
use kore::domains::mods;
use kore::domains::settings::{lang, nightly, ExceptionList, ScannerConfig, UpdateMode, WindowSettings};
#[cfg(target_os = "linux")]
use kore::domains::settings::desktop;
use kore::{ContentStore, Vault};

use crate::domains::home;
use crate::widget::popup;

use super::{logging, notice, updater, ActivePopup, BattleCatsApp, Message};

#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct WindowConfig {
    settings: SettingsWindowField,
}

#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct SettingsWindowField {
    window: WindowSettings,
}

pub(crate) fn saved_window_size() -> Size {
    let config: WindowConfig = json::load("settings.json").unwrap_or_default();
    Size::new(config.settings.window.width.max(800.0), config.settings.window.height.max(600.0))
}

fn split(phase: &mut Instant) -> u128 {
    let elapsed = phase.elapsed().as_millis();
    *phase = Instant::now();
    elapsed
}

impl BattleCatsApp {
    pub fn new() -> (Self, Task<Message>) {
        let boot = Instant::now();
        let mut phase = boot;

        let mut app: Self = json::load("settings.json").unwrap_or_default();
        app.app_state = json::load_state("state.json").unwrap_or_default();
        let settings_ms = split(&mut phase);

        logging::init_logging(app.settings.general.enable_logging);

        if let Some(note) = architecture::anchored() {
            info!("{}", note);
        }

        architecture::work_cleanup();

        info!(settings_ms, "Starting initialization sequence...");
        app.boot = Some(boot);

        app.cat_state.restore_state(&app.app_state.cat);
        app.enemy_state.restore_state(&app.app_state.enemy);
        app.stage_state.restore_state(&app.app_state.stage);
        app.mods_state.restore_state(&app.app_state.mods);
        app.files_state.restore_state(&app.app_state.files);
        app.help_state.restore_state(&app.app_state.help);
        let mounted = app.mods_state.active_mod();
        app.studio_state.restore_state(&app.app_state.studio, mounted.as_deref());
        popup::restore(&app.app_state.popups);

        app.studio_state.restore_onion(&mut app.settings);
        app.sync_popup(ActivePopup::StudioOnion, app.studio_state.onioning());

        if notice::should_show(&app.app_state.notice.acknowledged) {
            info!("Notice {} not yet acknowledged, showing at startup", notice::hash());
            app.notice_open = true;
            app.sync_popup(ActivePopup::VersionNotice, true);
        }

        if let Some(home) = architecture::volatile() {
            warn!(path = %home.display(), "Running from a temporary folder; imported data here can be cleared by the OS");
            app.init_errors.report_volatile(home);
            app.sync_popup(ActivePopup::InitErrors, true);
        }

        if let Some(state_dir) = dirs::state() {
            let _ = fs::remove_file(state_dir.join("meta.json"));
        }

        ExceptionList::sync_on_boot();
        info!(ms = split(&mut phase), "Boot phase: exception list synced");

        #[cfg(target_os = "linux")]
        {
            debug!("Syncing Linux desktop data");
            let _ = desktop::sync_desktop_data();
        }

        lang::ensure_complete_list(&mut app.settings.general.language_priority);
        nightly::settle(&mut app.settings.general.enable_nightly);

        debug!("Cleaning up temp update files");
        updater::cleanup_temp_files();
        info!(ms = split(&mut phase), "Boot phase: temp update files cleaned");

        let store_task = app.spawn_vault_build(true);
        info!(ms = split(&mut phase), "Boot phase: index build dispatched");

        let updater_task = if app.settings.general.update_mode != UpdateMode::Ignore {
            info!("Checking for app updates at startup");
            app.check_for_updates(false)
        } else {
            Task::none()
        };

        let (home_state, home_task) = home::State::new();
        app.home_state = home_state;

        let icon_streams = Task::batch([
            app.cat_state.icon_stream().map(Message::Cat),
            app.enemy_state.icon_stream().map(Message::Enemy),
            app.mods_state.icon_stream().map(Message::Mod),
        ]);

        let reveal_fallback = Task::future(Timer::after(super::WINDOW_SHOW_FALLBACK)).map(|_| Message::ShowWindow);

        info!(total_ms = boot.elapsed().as_millis(), "Initialization sequence complete");

        (app, Task::batch([home_task.map(Message::Home), updater_task, icon_streams, store_task, reveal_fallback]))
    }

    pub(super) fn spawn_vault_build(&mut self, hydrate: bool) -> Task<Message> {
        let config = self.settings.scanner_config(self.mods_state.active_mod());
        let (tx, rx) = mpsc::unbounded();

        self.rebuild_running = true;
        self.cat_state.set_indexing();
        self.enemy_state.set_indexing();
        self.stage_state.set_indexing();

        thread::spawn(move || {
            let stored = hydrate.then(|| {
                let mut cached = Vault::with_priority(&config.language_priority);
                let stored = hydrate_vault(&mut cached);
                let _ = tx.unbounded_send(Message::VaultHydrated(Arc::new(cached)));
                stored
            });

            let rebuilt = walk_vault(&config);
            let index = rebuilt.hash(config.active_mod.as_deref());
            let key = Some(Vault::key_for(index, &config));

            if stored == Some(Some(index)) {
                debug!(index, "File index still matches disk, keeping the hydrated caches");
                let _ = tx.unbounded_send(Message::VaultValidated { vault: None, key, mounted: config.active_mod });
                return;
            }

            info!(index, "The file index moved on disk, adopting the fresh walk");
            adopt_walk(&rebuilt, index);

            let vault = Some(Arc::new(rebuilt));
            let _ = tx.unbounded_send(Message::VaultValidated { vault, key, mounted: config.active_mod });
        });

        Task::stream(rx)
    }

    pub(super) fn report_conflicts(&mut self) {
        self.init_errors
            .report_conflicts(self.vault.vfs.conflicts(), self.settings.general.ignore_conflict_errors);
        self.sync_popup(ActivePopup::InitErrors, self.init_errors.is_open());
    }

    pub(super) fn adopt_vault(&mut self, vault: Arc<Vault>, cached: bool) -> Task<Message> {
        self.vault = vault;
        self.vault_ready = true;

        if !cached {
            self.report_conflicts();
        }

        self.sync_home_status();
        let files_task = self.files_state.sync(&self.vault.vfs, &self.settings.files, !cached);
        self.files_state.sync_state(&mut self.app_state.files);

        let active_mod = self.mods_state.active_mod();

        Task::batch([
            files_task.map(Message::Files),
            self.load_tables(),
            self.cat_state.start_load(&self.settings, &self.vault, active_mod.clone(), cached).map(Message::Cat),
            self.enemy_state.start_load(&self.settings, &self.vault, active_mod.clone(), cached).map(Message::Enemy),
            self.stage_state.start_load(&self.settings, &self.vault, active_mod, cached).map(Message::Stage),
        ])
    }
}

fn hydrate_vault(vault: &mut Vault) -> Option<u64> {
    let stored = vault.vfs.hydrate();

    if let Some(index) = stored {
        debug!(index, "Hydrated the file index from the virtual file system cache");
    }

    if let Some(content) = ContentStore::hydrate() {
        debug!("Hydrated parsed tables from the virtual data store cache");
        content.apply(&mut vault.vds);
    }

    stored
}

fn walk_vault(config: &ScannerConfig) -> Vault {
    let vault = Vault::with_priority(&config.language_priority);

    mount_game(&vault);

    if let Some(name) = config.active_mod.as_deref() {
        mount_mod(&vault, name);
    }

    vault
}

fn adopt_walk(vault: &Vault, index: u64) {
    if vault.vfs.count(architecture::GAME) == 0 {
        warn!("No game data left on disk, purging every derived cache");
        import::purge_derived_caches();
    }

    vault.vfs.persist(index);
}

fn mount_game(vault: &Vault) {
    info!("Indexing game data");

    match vault.vfs.create(Path::new(architecture::GAME)) {
        Ok(conflicts) => {
            for conflict in &conflicts {
                warn!(key = %conflict.key, "duplicate filename in game data, all copies excluded: {:?}", conflict.paths);
            }
        }
        Err(err) => error!("Failed to index game data: {}", err),
    }
}

fn mount_mod(vault: &Vault, name: &str) {
    info!(mod_name = name, "Mounting active mod");

    match mods::enable(vault, name) {
        Ok(conflicts) => {
            for conflict in &conflicts {
                warn!(key = %conflict.key, "duplicate filename in mod, all copies excluded: {:?}", conflict.paths);
            }
        }
        Err(err) => error!(mod_name = name, "Failed to mount active mod: {}", err),
    }
}
