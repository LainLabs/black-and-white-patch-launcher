//! Path rules shared with the decomp's setup program (src/setup/Install.cpp).

use std::path::{Component, Path, PathBuf};

/// True for a relative path that stays inside the folder it is relative to: no root, drive, stream, "." or ".."
/// component, or character Windows does not allow in file names. Same rule as `IsSafeRelativePath` in the decomp,
/// with "\" as the separator. Manifests are read from disk, so their paths are checked before anything is deleted.
pub fn is_safe_relative_path(path: &str) -> bool {
    if path.is_empty() || path.encode_utf16().count() >= 260 {
        return false;
    }
    path.split('\\').all(|part| {
        !part.is_empty()
            && part != "."
            && part != ".."
            && !part.ends_with('.')
            && !part.ends_with(' ')
            && part.chars().all(|c| c >= ' ' && !"<>:\"/|?*".contains(c))
    })
}

/// Lexical normalisation without touching the disk: removes "." and resolves ".." components.
pub fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// True when `a` and `b` name the same file, compared as Windows compares names (case-insensitive, trailing
/// separators ignored), like `SamePath` in the decomp.
pub fn same_path(a: &Path, b: &Path) -> bool {
    let text = |p: &Path| {
        let mut s = normalize(p).to_string_lossy().replace('/', "\\");
        while s.len() > 1 && s.ends_with('\\') && !s.ends_with(":\\") {
            s.pop();
        }
        s.to_lowercase()
    };
    text(a) == text(b)
}

/// The launcher's own folder for state and addons: %APPDATA%\BWLauncher.
pub fn state_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("BWLauncher")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_relative_paths() {
        assert!(is_safe_relative_path("BWDecomp.dll"));
        assert!(is_safe_relative_path(r"BWDecomp\README.txt"));
        for bad in ["", r"..\x", r"a\..\b", r"\x", r"\\server\x", r"C:\x", "a:stream", "a.", "a ", "a/b", r"a\\b"] {
            assert!(!is_safe_relative_path(bad), "{bad}");
        }
    }

    #[test]
    fn same_paths() {
        assert!(same_path(Path::new(r"C:\Games\"), Path::new(r"c:\games")));
        assert!(same_path(Path::new(r"C:\Games\x\..\y"), Path::new(r"C:\Games\y")));
        assert!(!same_path(Path::new(r"C:\Games\a"), Path::new(r"C:\Games\b")));
    }
}
