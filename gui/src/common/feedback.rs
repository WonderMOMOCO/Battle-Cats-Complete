use std::time::Duration;

use iced::{task, Task};

const EXPIRY: Duration = Duration::from_secs(2);

pub const CONFIRM_LABEL: &str = "Are You Sure?";

pub const CONFIRM_SHORT_LABEL: &str = "Confirm?";

pub const FAILURE_LABEL: &str = "Failed!";

pub const UNSUPPORTED_NOTICE: &str = "This action is not supported in the current app version";

pub const LOCKED_NOTICE: &str =
    "Vanilla \"game\" mount is locked and cannot be written\nMake a Mod or unlock under Settings > Files > Editor";

pub const MANIFEST_BUILDING_NOTICE: &str = "Import manifest building\nReverting to previous selection";

pub const MANIFEST_MISSING_NOTICE: &str = "Import manifest missing\nReverting to previous selection";

pub const MANIFEST_MALFORMED_NOTICE: &str = "Import manifest malformed\nReverting to previous selection";

pub const NO_ACTIONS_LABEL: &str = "No actions available";

pub const NO_ACTIONS_NOTICE: &str = "Couldn't find any actions within this Context Scope";

pub struct Slot<T> {
    value: Option<T>,
    handle: Option<task::Handle>,
}

impl<T> Default for Slot<T> {
    fn default() -> Self {
        Self { value: None, handle: None }
    }
}

impl<T> Slot<T> {
    pub fn set<M: Send + 'static>(&mut self, value: T, expired: M) -> Task<M> {
        self.set_after(value, expired, EXPIRY)
    }

    pub fn set_after<M: Send + 'static>(&mut self, value: T, expired: M, delay: Duration) -> Task<M> {
        self.value = Some(value);
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }

        let (task, handle) = Task::perform(
            async move {
                smol::Timer::after(delay).await;
            },
            move |_| expired,
        )
        .abortable();
        self.handle = Some(handle.abort_on_drop());
        task
    }

    pub fn expire(&mut self) {
        self.value = None;
        self.handle = None;
    }

    pub fn clear(&mut self) {
        self.value = None;
        self.handle = None;
    }

    pub fn get(&self) -> Option<&T> {
        self.value.as_ref()
    }

    pub fn is_set(&self) -> bool {
        self.value.is_some()
    }

    pub fn confirm_label<'a>(&self, idle: &'a str) -> &'a str {
        if self.is_set() { CONFIRM_LABEL } else { idle }
    }
}

impl<T: PartialEq> Slot<T> {
    pub fn armed_for(&self, value: &T) -> bool {
        self.value.as_ref() == Some(value)
    }

    pub fn take(&mut self, value: &T) -> bool {
        if !self.armed_for(value) {
            return false;
        }

        self.clear();
        true
    }
}
