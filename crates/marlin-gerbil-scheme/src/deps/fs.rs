//! Filesystem helpers for Gerbil package state and local toolchain repair.

use super::GerbilDepsError;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub(super) fn require_dir(path: &Path, label: impl AsRef<str>) -> Result<(), GerbilDepsError> {
    if path.is_dir() {
        Ok(())
    } else {
        Err(GerbilDepsError::message(format!(
            "missing {} directory at {}",
            label.as_ref(),
            path.display()
        )))
    }
}

pub(super) fn require_file(path: &Path, label: impl AsRef<str>) -> Result<(), GerbilDepsError> {
    if path.is_file() {
        Ok(())
    } else {
        Err(GerbilDepsError::message(format!(
            "missing {} at {}",
            label.as_ref(),
            path.display()
        )))
    }
}

pub(super) fn ensure_symlink(source: &Path, destination: &Path) -> Result<(), GerbilDepsError> {
    if destination.exists() || is_symlink(destination)? {
        return Ok(());
    }
    let parent = destination.parent().ok_or_else(|| {
        GerbilDepsError::message(format!("path has no parent: {}", destination.display()))
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        GerbilDepsError::message(format!("failed to create {}: {error}", parent.display()))
    })?;
    create_symlink(source, destination).map_err(|error| {
        GerbilDepsError::message(format!(
            "failed to symlink {} -> {}: {error}",
            destination.display(),
            source.display()
        ))
    })
}

pub(super) fn remove_existing_symlink(path: &Path) -> Result<(), GerbilDepsError> {
    if is_symlink(path)? {
        fs::remove_file(path).map_err(|error| {
            GerbilDepsError::message(format!(
                "failed to remove existing symlink {}: {error}",
                path.display()
            ))
        })
    } else if path.exists() {
        Err(GerbilDepsError::message(format!(
            "refusing to replace non-symlink Gerbil package path: {}",
            path.display()
        )))
    } else {
        Ok(())
    }
}

fn is_symlink(path: &Path) -> Result<bool, GerbilDepsError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.file_type().is_symlink()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(GerbilDepsError::message(format!(
            "failed to inspect {}: {error}",
            path.display()
        ))),
    }
}

#[cfg(unix)]
fn create_symlink(source: &Path, destination: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(source, destination)
}

#[cfg(windows)]
fn create_symlink(source: &Path, destination: &Path) -> io::Result<()> {
    if source.is_dir() {
        std::os::windows::fs::symlink_dir(source, destination)
    } else {
        std::os::windows::fs::symlink_file(source, destination)
    }
}

pub(super) fn package_destination(home_dir: &Path, package: &str) -> PathBuf {
    home_dir.join(".gerbil").join("pkg").join(package)
}
