#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OemDriver {
    Universal,
    Samsung,
    Google,
    Xiaomi,
    Lg,
    Motorola,
    Other,
}

impl Default for OemDriver {
    fn default() -> Self {
        Self::Universal
    }
}

pub struct OemManager {
    pub selected: OemDriver,
}

impl Default for OemManager {
    fn default() -> Self {
        Self {
            selected: OemDriver::Universal,
        }
    }
}

impl OemManager {
    pub fn all_drivers() -> Vec<OemDriver> {
        vec![
            OemDriver::Universal,
            OemDriver::Samsung,
            OemDriver::Google,
            OemDriver::Xiaomi,
            OemDriver::Lg,
            OemDriver::Motorola,
            OemDriver::Other,
        ]
    }

    pub fn label(driver: OemDriver) -> &'static str {
        match driver {
            OemDriver::Universal => "Universal",
            OemDriver::Samsung => "Samsung",
            OemDriver::Google => "Google (Pixel/Nexus)",
            OemDriver::Xiaomi => "Xiaomi / Redmi",
            OemDriver::Lg => "LG",
            OemDriver::Motorola => "Motorola",
            OemDriver::Other => "Other",
        }
    }

    pub fn execute_action(&self) {
        match self.selected {
            OemDriver::Universal => open_url("https://github.com/koush/adb.clockworkmod.com/releases/latest/download/UniversalAdbDriverSetup.msi"),
            OemDriver::Samsung => open_url("https://developer.samsung.com/mobile/android-usb-driver.html"),
            OemDriver::Google => open_url("https://developer.android.com/studio/run/win-usb"),
            OemDriver::Xiaomi => open_url("https://xiaomidriver.com/adb-driver"),
            OemDriver::Lg => open_url("https://www.lg.com/us/support?popup=software"),
            OemDriver::Motorola => open_url("https://support.motorola.com/us/en/solution/MS88481"),
            OemDriver::Other => open_url("https://developer.android.com/studio/run/oem-usb#Drivers"),
        }
    }
}

fn open_url(url: &str) {
    let _ = open::that(url);
}