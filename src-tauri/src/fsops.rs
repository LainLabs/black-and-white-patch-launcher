//! A list of file operations inside one folder (the game folder), run directly or, when the folder needs them,
//! with administrator rights in one go (one UAC prompt). Used for settings and addons. Every target is a safe
//! relative path inside the root; sources (temporary files, addon content) can be anywhere. Each operation can be
//! repeated, so a list that failed part way for lack of rights is simply run again elevated.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::elevate::{self, Request};
use crate::error::{Context, Error, Result};
use crate::install::write_replacing;
use crate::paths::is_safe_relative_path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FileOp {
    /// Copies `from` (absolute) over `to` (relative to the root), creating folders.
    Copy { from: PathBuf, to: String },
    /// Deletes a file; nothing when it does not exist.
    Delete { path: String },
    /// Removes a folder if it is empty.
    RemoveDirIfEmpty { path: String },
}

fn target(root: &Path, relative: &str) -> Result<PathBuf> {
    if !is_safe_relative_path(relative) {
        return Err(Error::msg(format!("Refusing a path outside the game folder: {relative}")));
    }
    Ok(root.join(relative))
}

pub fn apply(root: &Path, ops: &[FileOp]) -> Result<()> {
    if !root.is_dir() {
        return Err(Error::msg(format!("The folder {} does not exist.", root.display())));
    }
    for op in ops {
        match op {
            FileOp::Copy { from, to } => {
                let to = target(root, to)?;
                let data = std::fs::read(from).context("Cannot read", from)?;
                if let Some(parent) = to.parent() {
                    std::fs::create_dir_all(parent).context("Cannot create the folder", parent)?;
                }
                write_replacing(&to, &data)?;
            }
            FileOp::Delete { path } => {
                let path = target(root, path)?;
                match std::fs::remove_file(&path) {
                    Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                        return Err(e).context("Cannot delete", &path);
                    }
                    _ => {}
                }
            }
            FileOp::RemoveDirIfEmpty { path } => {
                let _ = std::fs::remove_dir(target(root, path)?);
            }
        }
    }
    Ok(())
}

/// Runs `ops`, and again with administrator rights if this process may not write in the folder.
pub fn apply_with_rights(root: &Path, ops: Vec<FileOp>) -> Result<()> {
    match apply(root, &ops) {
        Err(e) if e.is_access_denied() && !elevate::is_elevated() => {
            elevate::execute_with_rights(&Request::FileOps { root: root.to_path_buf(), ops }, true).map(|_| ())
        }
        other => other,
    }
}

/// A scratch file in the launcher's folder holding `data`, as the source of a Copy.
pub fn stage(data: &[u8]) -> Result<PathBuf> {
    let dir = crate::paths::state_dir().join("staging");
    std::fs::create_dir_all(&dir)?;
    let name = format!(
        "{}-{}.tmp",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
    );
    let path = dir.join(name);
    std::fs::write(&path, data)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_deletes_and_refuses_escapes() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.txt");
        std::fs::write(&source, b"data").unwrap();
        let root = dir.path().join("game");
        std::fs::create_dir(&root).unwrap();

        apply(&root, &[FileOp::Copy { from: source.clone(), to: r"Textures\Replace\pack\a.png".into() }]).unwrap();
        assert_eq!(std::fs::read(root.join(r"Textures\Replace\pack\a.png")).unwrap(), b"data");

        apply(
            &root,
            &[
                FileOp::Delete { path: r"Textures\Replace\pack\a.png".into() },
                FileOp::RemoveDirIfEmpty { path: r"Textures\Replace\pack".into() },
            ],
        )
        .unwrap();
        assert!(!root.join(r"Textures\Replace\pack").exists());

        assert!(apply(&root, &[FileOp::Delete { path: r"..\source.txt".into() }]).is_err());
        assert!(source.exists());
    }
}
