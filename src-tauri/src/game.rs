//! Finding the game and recognising its runblack.exe. Mirrors the decomp's setup program:
//! `FindGameFolder` and `IsGameRunning` (src/setup/System.cpp), `CheckGameExecutable` and `DescribeGameExecutable`
//! (src/core/GameExecutable.cpp).

use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{paths, payload, registry};

pub const GAME_EXE: &str = "runblack.exe";
pub const LAUNCH_EXE: &str = "BWLaunch.exe";
/// runblack.exe is 8 MB; anything far larger is not it.
const LARGEST_EXECUTABLE: u64 = 64 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GameBuild {
    /// No such file.
    Missing,
    /// Exists but cannot be read (or is implausibly large).
    Unreadable,
    /// The unprotected 1.00 build the patch supports.
    Supported,
    /// The same build wrapped in SafeDisc: cannot start on current Windows.
    RetailSafeDisc,
    /// Another version or language.
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameCheck {
    pub folder: String,
    pub build: GameBuild,
    /// Lower-case hex; empty for Missing and Unreadable.
    pub sha256: String,
    /// Why the game cannot be started, for the player; empty for the supported build.
    pub message: String,
}

pub fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect()
}

/// SHA-256 of a whole file; None when it cannot be read or is larger than `largest` bytes.
pub fn file_sha256(path: &Path, largest: u64) -> Option<String> {
    let size = std::fs::metadata(path).ok()?.len();
    if size > largest {
        return None;
    }
    std::fs::read(path).ok().map(|data| sha256_hex(&data))
}

pub fn holds_game(folder: &Path) -> bool {
    !folder.as_os_str().is_empty() && folder.join(GAME_EXE).is_file()
}

pub fn check_executable(folder: &Path) -> GameCheck {
    let file = folder.join(GAME_EXE);
    let (build, sha256) = if !file.is_file() {
        (GameBuild::Missing, String::new())
    } else {
        match file_sha256(&file, LARGEST_EXECUTABLE) {
            None => (GameBuild::Unreadable, String::new()),
            Some(hash) if hash == payload::SUPPORTED_SHA256 => (GameBuild::Supported, hash),
            Some(hash) if hash == payload::RETAIL_SAFEDISC_SHA256 => (GameBuild::RetailSafeDisc, hash),
            Some(hash) => (GameBuild::Unknown, hash),
        }
    };
    let message = describe(build, &sha256, &file);
    GameCheck { folder: folder.display().to_string(), build, sha256, message }
}

fn describe(build: GameBuild, sha256: &str, file: &Path) -> String {
    match build {
        GameBuild::Supported => String::new(),
        GameBuild::Missing => format!("Cannot find {}. Choose the folder Black & White is installed in.", file.display()),
        GameBuild::Unreadable => format!("Cannot read {}.", file.display()),
        GameBuild::RetailSafeDisc => "This runblack.exe is the SafeDisc-protected retail build. SafeDisc needs the \
            secdrv.sys driver, which current Windows versions refuse to load, so this file cannot start.\n\n\
            The patch targets the unprotected build of the same version that is on the Black & White disc \
            (crack\\runblack.exe). Copy that file over the installed runblack.exe and check again."
            .to_string(),
        GameBuild::Unknown => format!(
            "Unsupported runblack.exe (SHA-256 {sha256}).\n\nThe patch only supports Black & White 1.00 \
             (English) with SHA-256 {}. The official patches 1.1 and 1.2 replace runblack.exe with builds that are \
             not supported yet.",
            payload::SUPPORTED_SHA256
        ),
    }
}

/// The game folder, in the order the decomp's setup looks: the folder the launcher remembers, an earlier setup
/// installation, the game's GameDir (current user, then machine), the retail default below Program Files.
pub fn find_game_folder(remembered: Option<&Path>) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    candidates.extend(remembered.map(Path::to_path_buf));
    candidates.extend(registry::legacy_install_location());
    candidates.extend(registry::game_dir_candidates());
    for var in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(programs) = std::env::var_os(var) {
            candidates.push(PathBuf::from(programs).join("Lionhead Studios Ltd").join("Black & White"));
        }
    }
    candidates.into_iter().find(|c| holds_game(c)).map(|c| paths::normalize(&c))
}

/// True while runblack.exe or BWLaunch.exe runs from `folder`.
#[cfg(windows)]
pub fn is_game_running(folder: &Path) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return false;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = false;
        let mut more = Process32FirstW(snapshot, &mut entry) != 0;
        while more && !found {
            let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
            if name.eq_ignore_ascii_case(GAME_EXE) || name.eq_ignore_ascii_case(LAUNCH_EXE) {
                let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, entry.th32ProcessID);
                if !process.is_null() {
                    let mut buffer = vec![0u16; 32768];
                    let mut size = buffer.len() as u32;
                    if QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut size) != 0 {
                        let image = PathBuf::from(String::from_utf16_lossy(&buffer[..size as usize]));
                        found = image.parent().is_some_and(|parent| paths::same_path(parent, folder));
                    }
                    CloseHandle(process);
                }
            }
            more = Process32NextW(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_executables() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(check_executable(dir.path()).build, GameBuild::Missing);
        std::fs::write(dir.path().join(GAME_EXE), b"not the game").unwrap();
        let check = check_executable(dir.path());
        assert_eq!(check.build, GameBuild::Unknown);
        assert_eq!(check.sha256, sha256_hex(b"not the game"));
        assert!(check.message.contains(&check.sha256));
    }

    #[test]
    fn sha256_known_value() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
}
