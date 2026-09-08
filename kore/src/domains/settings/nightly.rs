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

// The setting outlives the features it unlocked: once the last one graduates, a `true` saved
// from a previous run would keep gating pages open with nothing behind the gate. Settled at
// boot rather than only when the Settings tab is opened, which a session may never do.
pub fn settle(enabled: &mut bool) {
    if features_available() || !*enabled {
        return;
    }

    info!("No feature needs the nightly gate, switching it back off");

    *enabled = false;
}