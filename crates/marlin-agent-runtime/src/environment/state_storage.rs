// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Initializes runtime-owned state-home directories.

use std::{
    fs,
    path::{Component, Path},
};

use marlin_agent_protocol::{
    RuntimeStateLayout, RuntimeStateStorageReceipt, RuntimeStateStorageStatus,
};

/// Filesystem initializer for the unified runtime state home.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeStateStorageInitializer;

impl RuntimeStateStorageInitializer {
    pub fn new() -> Self {
        Self
    }

    pub fn ensure_layout(&self, layout: &RuntimeStateLayout) -> RuntimeStateStorageReceipt {
        if let Err(reason) = validate_state_home_root(&layout.home.path) {
            return RuntimeStateStorageReceipt::failed(layout.home.clone(), reason);
        }
        for directory in &layout.directories {
            if let Err(reason) =
                validate_state_directory_path(&layout.home.path, &directory.path)
            {
                return RuntimeStateStorageReceipt::failed(layout.home.clone(), reason);
            }
            if let Err(error) = fs::create_dir_all(&directory.path) {
                return RuntimeStateStorageReceipt::failed(
                    layout.home.clone(),
                    format!(
                        "failed to create {} state directory at {}: {error}",
                        directory.kind.relative_path(),
                        directory.path.display()
                    ),
                );
            }
        }

        let receipt = RuntimeStateStorageReceipt::initialized(layout);
        debug_assert_eq!(receipt.status, RuntimeStateStorageStatus::Initialized);
        receipt
    }
}

fn validate_state_home_root(home_root: &Path) -> Result<(), String> {
    if home_root
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(format!(
            "state home path `{}` contains parent traversal",
            home_root.display()
        ));
    }
    match fs::symlink_metadata(home_root) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => Err(format!(
            "state home path `{}` must be a real directory, not a symlink",
            home_root.display()
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "failed to inspect state home `{}`: {error}",
            home_root.display()
        )),
    }
}

fn validate_state_directory_path(home_root: &Path, directory_path: &Path) -> Result<(), String> {
    let relative = directory_path.strip_prefix(home_root).map_err(|_| {
        format!(
            "state directory `{}` escapes state home `{}`",
            directory_path.display(),
            home_root.display()
        )
    })?;
    let mut current = home_root.to_path_buf();
    for component in relative.components() {
        match component {
            Component::Normal(part) => {
                current.push(part);
                match fs::symlink_metadata(&current) {
                    Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                        return Err(format!(
                            "state directory component `{}` must be a real directory, not a symlink",
                            current.display()
                        ));
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => {
                        return Err(format!(
                            "failed to inspect state directory `{}`: {error}",
                            current.display()
                        ));
                    }
                }
            }
            Component::CurDir => {}
            _ => {
                return Err(format!(
                    "state directory `{}` contains disallowed path traversal",
                    directory_path.display()
                ));
            }
        }
    }
    Ok(())
}
