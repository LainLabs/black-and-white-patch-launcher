//! Display modes for the resolution picker.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayModes {
    /// Distinct sizes of the primary display, largest first.
    pub modes: Vec<DisplayMode>,
    pub current: Option<DisplayMode>,
}

pub fn display_modes() -> DisplayModes {
    use windows_sys::Win32::Graphics::Gdi::{EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS};
    let mut modes = Vec::new();
    let mut current = None;
    unsafe {
        let mut mode: DEVMODEW = std::mem::zeroed();
        mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        let mut i = 0;
        while EnumDisplaySettingsW(std::ptr::null(), i, &mut mode) != 0 {
            if mode.dmBitsPerPel >= 16 && mode.dmPelsWidth >= 640 && mode.dmPelsHeight >= 480 {
                modes.push(DisplayMode { width: mode.dmPelsWidth, height: mode.dmPelsHeight });
            }
            i += 1;
        }
        if EnumDisplaySettingsW(std::ptr::null(), ENUM_CURRENT_SETTINGS, &mut mode) != 0 {
            current = Some(DisplayMode { width: mode.dmPelsWidth, height: mode.dmPelsHeight });
        }
    }
    modes.sort_by(|a, b| b.cmp(a));
    modes.dedup();
    DisplayModes { modes, current }
}
