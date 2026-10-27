//! Administrator rights, only when the game folder needs them (e.g. below Program Files), as the decomp's setup
//! does: probe write access, and if it is denied run a copy of the launcher with `runas` that does only the file
//! work. Registry values and launcher state stay with the player's own account.
//!
//! Elevated command line: `bw-launcher.exe --elevated <request.json> <result.json>`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::fsops::FileOp;
use crate::{fsops, install, payload, updates};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum Request {
    Install { folder: PathBuf },
    /// A newer patch from the patch's releases, already checked against its SHA-256 sum.
    InstallZip { folder: PathBuf, zip: PathBuf, version: String },
    Remove { folder: PathBuf },
    FileOps { root: PathBuf, ops: Vec<FileOp> },
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    pub error: String,
    pub data: serde_json::Value,
}

/// Runs a request in this process: the work the elevated copy does, and what runs directly when no rights are needed.
pub fn execute(request: &Request) -> Result<serde_json::Value> {
    match request {
        Request::Install { folder } => {
            let files: Vec<install::InstallFile> = payload::FILES
                .iter()
                .map(|f| install::InstallFile { path: f.path, keep: f.keep, data: f.data })
                .collect();
            install::install_files(folder, &files, payload::DECOMP_VERSION)?;
            Ok(serde_json::Value::Null)
        }
        Request::InstallZip { folder, zip, version } => {
            updates::install_patch_zip(folder, zip, version)?;
            Ok(serde_json::Value::Null)
        }
        Request::Remove { folder } => Ok(serde_json::to_value(install::remove_files(folder)?)?),
        Request::FileOps { root, ops } => {
            fsops::apply(root, ops)?;
            Ok(serde_json::Value::Null)
        }
    }
}

/// Runs `request` here, or elevated when this process cannot write where it needs to.
pub fn execute_with_rights(request: &Request, needs_rights: bool) -> Result<serde_json::Value> {
    if !needs_rights || is_elevated() {
        return execute(request);
    }
    run_elevated(request)
}

/// Entry point of the elevated copy. Returns the process exit code.
pub fn elevated_main(request_file: &Path, result_file: &Path) -> i32 {
    let response = match std::fs::read(request_file)
        .map_err(Error::from)
        .and_then(|text| Ok(serde_json::from_slice::<Request>(&text)?))
        .and_then(|request| execute(&request))
    {
        Ok(data) => Response { ok: true, error: String::new(), data },
        Err(e) => Response { ok: false, error: e.to_string(), data: serde_json::Value::Null },
    };
    let written = serde_json::to_vec(&response).map(|bytes| std::fs::write(result_file, bytes));
    if response.ok && matches!(written, Ok(Ok(()))) {
        0
    } else {
        1
    }
}

fn run_elevated(request: &Request) -> Result<serde_json::Value> {
    let dir = crate::paths::state_dir().join("elevated");
    std::fs::create_dir_all(&dir)?;
    let stamp = format!("{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos());
    let request_file = dir.join(format!("request-{stamp}.json"));
    let result_file = dir.join(format!("result-{stamp}.json"));
    std::fs::write(&request_file, serde_json::to_vec(request)?)?;
    let exe = std::env::current_exe()?;
    let arguments = [
        "--elevated".to_string(),
        request_file.display().to_string(),
        result_file.display().to_string(),
    ];
    let launched = shell_execute_runas(&exe, &arguments);
    let response = std::fs::read(&result_file).ok().and_then(|b| serde_json::from_slice::<Response>(&b).ok());
    let _ = std::fs::remove_file(&request_file);
    let _ = std::fs::remove_file(&result_file);
    launched?;
    match response {
        Some(Response { ok: true, data, .. }) => Ok(data),
        Some(Response { error, .. }) => Err(Error::msg(error)),
        None => Err(Error::msg("The task with administrator rights ended without a result.")),
    }
}

/// Quotes one argument for CommandLineToArgvW (backslashes before quotes are doubled), as the decomp's QuoteArgument.
pub fn quote_argument(argument: &str) -> String {
    if !argument.is_empty() && !argument.contains([' ', '\t', '\n', '\x0b', '"']) {
        return argument.to_string();
    }
    let mut out = String::from("\"");
    let mut backslashes = 0;
    for c in argument.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                out.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                out.extend(std::iter::repeat_n('\\', backslashes));
                out.push(c);
                backslashes = 0;
            }
        }
    }
    out.extend(std::iter::repeat_n('\\', backslashes * 2));
    out.push('"');
    out
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn shell_execute_runas(exe: &Path, arguments: &[String]) -> Result<()> {
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
    use windows_sys::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let verb = wide("runas");
    let file = wide(&exe.display().to_string());
    let parameters = wide(&arguments.iter().map(|a| quote_argument(a)).collect::<Vec<_>>().join(" "));
    unsafe {
        let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
        info.lpVerb = verb.as_ptr();
        info.lpFile = file.as_ptr();
        info.lpParameters = parameters.as_ptr();
        info.nShow = SW_HIDE;
        if ShellExecuteExW(&mut info) == 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_CANCELLED as i32) {
                return Err(Error::msg("Administrator rights are needed to change the game folder, and were refused."));
            }
            return Err(Error::msg(format!("Cannot start the task with administrator rights: {error}")));
        }
        if info.hProcess.is_null() {
            return Err(Error::msg("Cannot start the task with administrator rights."));
        }
        let waited = WaitForSingleObject(info.hProcess, INFINITE);
        let mut code = 1u32;
        GetExitCodeProcess(info.hProcess, &mut code);
        CloseHandle(info.hProcess);
        if waited != WAIT_OBJECT_0 {
            return Err(Error::msg("Lost track of the task with administrator rights."));
        }
    }
    Ok(())
}

pub fn is_elevated() -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut size = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            (&mut elevation as *mut TOKEN_ELEVATION).cast(),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        );
        CloseHandle(token);
        ok != 0 && elevation.TokenIsElevated != 0
    }
}

/// False when this process may not write `path`: a folder (by creating a file) or an existing file (by opening it
/// for replacement). Like CanWrite in the decomp's setup.
pub fn can_write(path: &Path) -> bool {
    use std::io::ErrorKind::PermissionDenied;
    if path.is_dir() {
        let probe = path.join(format!(".bwlauncher-{}", std::process::id()));
        let result = std::fs::OpenOptions::new().write(true).create_new(true).open(&probe);
        let _ = std::fs::remove_file(&probe);
        return !matches!(result, Err(e) if e.kind() == PermissionDenied);
    }
    if path.exists() {
        let result = std::fs::OpenOptions::new().write(true).open(path);
        return !matches!(result, Err(e) if e.kind() == PermissionDenied);
    }
    true
}

/// True when installing into `folder` needs administrator rights.
pub fn install_needs_rights(folder: &Path) -> bool {
    if is_elevated() {
        return false;
    }
    let mut paths = vec![folder.to_path_buf(), folder.join(install::DATA_FOLDER)];
    paths.extend(payload::FILES.iter().map(|f| folder.join(f.path)));
    paths.push(folder.join(install::MANIFEST_PATH));
    !paths.iter().all(|p| can_write(p))
}

#[cfg(test)]
mod tests {
    use super::quote_argument;

    #[test]
    fn quoting() {
        assert_eq!(quote_argument("plain"), "plain");
        assert_eq!(quote_argument(""), "\"\"");
        assert_eq!(quote_argument(r"C:\Program Files\x"), r#""C:\Program Files\x""#);
        assert_eq!(quote_argument(r"C:\a b\"), r#""C:\a b\\""#);
        assert_eq!(quote_argument(r#"say "hi""#), r#""say \"hi\"""#);
    }
}
