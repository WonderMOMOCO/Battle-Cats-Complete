use std::ffi::OsStr;

use super::*;

impl State {
    pub(super) fn sealed(&self) -> bool {
        self.manage.sealed()
    }

    fn mount(&self) -> Option<String> {
        let session = self.session.as_ref().filter(|_| self.sealed())?;

        Some(State::mount_of(&session.plan))
    }

    pub(crate) fn unlatch(&mut self) {
        if !self.sealed() {
            return;
        }

        self.flush_now();
        self.stash();
        self.session = None;
        self.manage.adopt(sets::Set::default());
    }

    fn foldered(&self) -> bool {
        sets::folder_name(self.manage.set()).is_some()
    }

    fn tracked(&self) -> Option<&str> {
        self.session
            .as_ref()?
            .viewer
            .selected_anim()?
            .file_stem()
            .and_then(OsStr::to_str)
    }

    fn seedable(&self) -> bool {
        let Some(model) = self.manage.set().model.as_deref() else {
            return false;
        };

        sets::home(model) != sets::Home::Game || self.unlocked
    }

    pub(super) fn manage(&mut self, message: manage::Message) -> Task<Message> {
        match message {
            manage::Message::NameChanged(typed) => {
                if self.sealed() {
                    return Task::none();
                }

                self.manage.rename(typed);

                self.manage
                    .renamer
                    .set_after((), Message::Manage(manage::Message::Rename), RENAME_DELAY)
            }
            manage::Message::Rename => {
                self.manage.renamer.expire();
                self.resettle(Swap::Same)
            }
            manage::Message::Import => {
                if self.manage.set().rigged() && !self.manage.importing.take(&()) {
                    return self
                        .manage
                        .importing
                        .set((), Message::Manage(manage::Message::ImportExpired));
                }

                Task::perform(dialog::file("Animation Set", &["png", "imgcut", "mamodel"]), |picked| {
                    Message::Manage(manage::Message::Imported(picked))
                })
            }
            manage::Message::ImportExpired => {
                self.manage.importing.expire();

                Task::none()
            }
            manage::Message::Imported(picked) => {
                let Some(source) = picked else {
                    return Task::none();
                };

                self.manage.adopt(sets::siblings(&source));

                self.resettle(Swap::Fresh)
            }
            manage::Message::New => {
                let name = sets::vacant(sets::SEED_NAME);

                match sets::seed(&name) {
                    Ok(set) => self.manage.adopt(set),
                    Err(err) => {
                        warn!(name, "Studio could not seed a new set: {}", err);

                        return Task::none();
                    }
                }

                self.resettle(Swap::Fresh)
            }
            manage::Message::Recall(name) => {
                let picked = match name == manage::NONE_ENTRY {
                    true => sets::Set::default(),
                    false => sets::load(&name),
                };

                self.manage.adopt(picked);

                self.resettle(Swap::Fresh)
            }
            manage::Message::Pick(slot) => {
                if self.manage.set().slot(slot).is_some() && !self.manage.picking.take(&slot) {
                    return self
                        .manage
                        .picking
                        .set(slot, Message::Manage(manage::Message::PickExpired));
                }

                Task::perform(dialog::file(slot.label(), slot.filter()), move |picked| {
                    Message::Manage(manage::Message::Picked(slot, picked))
                })
            }
            manage::Message::PickExpired => {
                self.manage.picking.expire();

                Task::none()
            }
            manage::Message::Picked(slot, picked) => {
                let Some(source) = picked else {
                    return Task::none();
                };

                let seated = self.manage.set().slot(slot).map(Path::to_path_buf);
                let replaceable = seated.as_deref().is_some_and(|held| sets::home(held) != sets::Home::Loose);

                let Some(seated) = seated.filter(|_| replaceable) else {
                    self.manage.place(slot, source);

                    return self.resettle(Swap::Same);
                };

                if source == seated {
                    return Task::none();
                }

                if let Some(session) = self.session.as_mut() {
                    session.remember(Tag::Bulk);
                }

                if let Err(err) = fs::copy(&source, &seated) {
                    warn!(path = %seated.display(), "Studio could not replace the file: {}", err);

                    return Task::none();
                }

                self.reopen()
            }
            manage::Message::TrackChanged(typed) => {
                self.manage.retrack(typed);

                self.manage
                    .tracker
                    .set_after((), Message::Manage(manage::Message::TrackRename), RENAME_DELAY)
            }
            manage::Message::TrackRename => {
                self.manage.tracker.expire();

                self.rename_track()
            }
            manage::Message::NewAnim => {
                self.manage.dropping.clear();
                self.settle_folder();

                let seeded = match sets::seed_track(self.manage.set()) {
                    Ok(path) => path,
                    Err(err) => {
                        warn!("Studio could not seed a new track: {}", err);

                        return Task::none();
                    }
                };

                let stem = seeded.file_stem().and_then(OsStr::to_str).map(str::to_owned);

                self.manage.anims_mut().push(seeded);
                self.aim_clip(stem);

                self.resettle(Swap::Same)
            }
            manage::Message::AddAnims => {
                Task::perform(dialog::files("Animation", &["maanim"]), |picked| {
                    Message::Manage(manage::Message::AnimsPicked(picked))
                })
            }
            manage::Message::AnimsPicked(picked) => {
                if picked.is_empty() {
                    return Task::none();
                }

                for path in picked {
                    if !self.manage.set().anims.contains(&path) {
                        self.manage.anims_mut().push(path);
                    }
                }

                self.resettle(Swap::Same)
            }
            manage::Message::DropAnim => {
                if !self.manage.dropping.take(&()) {
                    return self.manage.dropping.set((), Message::Manage(manage::Message::DropExpired));
                }

                let Some(held) =
                    self.session.as_ref().and_then(|session| session.viewer.selected_anim().cloned())
                else {
                    return Task::none();
                };

                self.flush_now();

                if sets::home(&held) == sets::Home::Studio
                    && let Err(err) = fs::remove_file(&held)
                {
                    warn!(path = %held.display(), "Studio could not delete the track: {}", err);
                }

                self.manage.anims_mut().retain(|path| *path != held);

                self.resettle(Swap::Same)
            }
            manage::Message::DropExpired => {
                self.manage.dropping.expire();

                Task::none()
            }
            manage::Message::Reveal => {
                let Some(folder) = sets::folder_name(self.manage.set()).map(|name| sets::root().join(name))
                else {
                    return Task::none();
                };

                if let Err(err) = open::that(&folder) {
                    warn!(path = %folder.display(), "Studio could not open the set folder: {}", err);
                }

                Task::none()
            }
            manage::Message::WipeSet => {
                if !self.manage.wiping.take(&()) {
                    return self.manage.wiping.set((), Message::Manage(manage::Message::WipeExpired));
                }

                let Some(name) = sets::folder_name(self.manage.set()) else {
                    return Task::none();
                };

                self.flush_now();
                self.session = None;

                if let Err(err) = sets::discard(&name) {
                    warn!(name, "Studio could not delete the set: {}", err);

                    return Task::none();
                }

                self.manage.adopt(sets::Set::default());
                self.manage.restock();

                self.resettle(Swap::Fresh)
            }
            manage::Message::WipeExpired => {
                self.manage.wiping.expire();

                Task::none()
            }
        }
    }

    fn resettle(&mut self, swap: Swap) -> Task<Message> {
        self.settle_folder();

        if !self.manage.set().rigged() {
            self.flush_now();
            self.stash();
            self.session = None;

            return Task::none();
        }

        if self.session.as_ref().is_some_and(|session| session.plan.set == *self.manage.set()) {
            return Task::none();
        }

        let wanted = self.manage.set().clone();

        if swap == Swap::Same
            && let Some(session) = self.session.as_mut()
        {
            session.reseat(wanted);
            self.manage.restock();

            return Task::none();
        }

        let target_mod = self.session.as_ref().and_then(|session| session.plan.target_mod.clone());
        let clip = self.session.as_ref().and_then(|session| session.viewer.selected_label());

        self.begin(plan(wanted, target_mod, clip));
        self.manage.restock();

        Task::none()
    }

    fn reopen(&mut self) -> Task<Message> {
        if let Some(session) = self.session.as_mut() {
            session.refresh();
        }

        Task::none()
    }

    pub(super) fn settle_track(&mut self) {
        if self.manage.typed_track() == self.tracked() {
            self.manage.untrack();
        }
    }

    fn aim_clip(&mut self, label: Option<String>) {
        let Some(session) = self.session.as_mut() else {
            return;
        };

        session.plan.clip = label;
        session.primed = false;
    }

    fn rename_track(&mut self) -> Task<Message> {
        let Some(typed) = self.manage.typed_track().map(str::to_owned) else {
            return Task::none();
        };

        let held = self.session.as_ref().and_then(|session| session.viewer.selected_anim().cloned());

        let Some(held) = held else {
            self.manage.untrack();

            return Task::none();
        };

        self.flush_now();

        let renamed = match sets::rename_track(&held, &typed) {
            Ok(path) => path,
            Err(err) => {
                warn!(path = %held.display(), "Studio could not rename the track: {}", err);
                self.manage.untrack();

                return Task::none();
            }
        };

        if renamed == held {
            self.manage.untrack();

            return Task::none();
        }

        let stem = renamed.file_stem().and_then(OsStr::to_str).map(str::to_owned);

        for path in self.manage.anims_mut().iter_mut() {
            if *path == held {
                *path = renamed.clone();
            }
        }

        if let Some(stem) = stem.clone() {
            self.manage.retrack(stem);
        }

        self.aim_clip(self.manage.set().slot_label(&renamed).or(stem));

        self.resettle(Swap::Same)
    }

    fn settle_folder(&mut self) {
        if self.manage.set().files().is_empty() || self.sealed() {
            return;
        }

        self.pull_in();
        self.rename_folder();
    }

    fn pull_in(&mut self) {
        if !sets::pullable(self.manage.set(), self.unlocked) {
            return;
        }

        let name = sets::folder_name(self.manage.set())
            .unwrap_or_else(|| sets::vacant(&sets::sanitize(self.manage.name())));

        match sets::adopt(&name, self.manage.set()) {
            Ok(adopted) => self.manage.adopt(adopted),
            Err(err) => warn!(name, "Studio could not adopt the set: {}", err),
        }
    }

    fn rename_folder(&mut self) {
        let Some(held) = sets::folder_name(self.manage.set()) else {
            return;
        };

        let wanted = sets::sanitize(self.manage.name());

        if wanted == held {
            return;
        }

        let name = sets::vacant(&wanted);

        if let Err(err) = sets::rename(&held, &name) {
            warn!(from = %held, to = %name, "Studio could not rename the set folder: {}", err);

            return;
        }

        self.manage.adopt(sets::load(&name));
    }

    pub(crate) fn managing(&self) -> bool {
        self.managing
    }

    pub(crate) fn remount(&mut self, mounted: Option<String>) {
        if self.mounted != mounted {
            self.shipping_to = false;
        }

        self.mounted = mounted;
        self.unlatch();
    }

    pub(crate) fn shipping_to(&self) -> bool {
        self.shipping_to
    }

    pub(crate) fn aim_at(&mut self, muster: &shipout::Muster<'_>) {
        self.aim = sets::resolve(&self.aimed, muster);
    }

    pub(crate) fn ship_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        if !self.shipping_to {
            return None;
        }

        let named = match self.manage.set().name.trim() {
            "" => sets::DEFAULT_NAME,
            held => held,
        };

        Some(self.ship_popup.view(
            shipout::TITLE,
            shipout::SPEC,
            window,
            Message::ShipPopup,
            || shipout::view(named, &self.aimed, &self.aim, self.ship_armed.armed_for(&()), self.shipping()),
            Some(popup::GLASS),
        ))
    }

    pub(crate) fn onioning(&self) -> bool {
        self.onioning
    }

    pub(crate) fn restore_onion(&mut self, settings: &mut Settings) {
        self.onioning = settings.studio.onion.on();

        if self.onioning {
            settings.studio.onion_arm(true);
        }
    }

    pub(crate) fn onion_popup_view<'a>(
        &'a self,
        settings: &'a Settings,
        window: Size,
    ) -> Option<Element<'a, Message>> {
        self.onioning.then(|| {
            self.onion_popup.view(
                onion::TITLE,
                onion::SPEC,
                window,
                Message::OnionPopup,
                || onion::view(&settings.studio),
                Some(popup::GLASS),
            )
        })
    }

    pub(crate) fn manage_popup_view(&self, window: Size) -> Option<Element<'_, Message>> {
        self.managing.then(|| {
            self.popup.view(
                manage::TITLE,
                manage::SPEC,
                window,
                Message::ManagePopup,
                || {
                    self.manage
                        .view(self.mount(), self.tracked(), self.seedable(), self.foldered())
                        .map(Message::Manage)
                },
                None,
            )
        })
    }
}
