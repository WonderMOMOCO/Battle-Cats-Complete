use std::sync::atomic::{AtomicBool, Ordering};

use tracing::info;

static NIGHTLY_FEATURES_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn register_nightly_usage() {
    info!("Nightly development features activated.");
    NIGHTLY_FEATURES_ACTIVE.store(true, Ordering::Relaxed);
}

pub fn features_available() -> bool {
    NIGHTLY_FEATURES_ACTIVE.load(Ordering::Relaxed)
}

pub fn settle(enabled: &mut bool) {
    if features_available() || !*enabled {
        return;
    }

    info!("No feature needs the nightly gate, switching it back off");

    *enabled = false;
}