//! The decompilation's files, embedded by build.rs from payload/.

pub struct PayloadFile {
    /// Relative to the game folder, "\" separated.
    pub path: &'static str,
    /// The player's settings: installed only when missing, removed only when unchanged.
    pub keep: bool,
    pub sha256: &'static str,
    pub data: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/payload.rs"));

pub fn has_payload() -> bool {
    !FILES.is_empty()
}

/// The shipped settings, used for "Reset to defaults" and when the game folder has no BWDecomp.ini.
pub fn default_ini() -> Option<&'static str> {
    FILES
        .iter()
        .find(|f| f.path.eq_ignore_ascii_case("BWDecomp.ini"))
        .and_then(|f| std::str::from_utf8(f.data).ok())
}
