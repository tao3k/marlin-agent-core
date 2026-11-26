// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Resolve a local development Scheme package before the terminal runtime starts.

#[cfg(any(test, all(unix, not(feature = "linked-ui"))))]
fn has_marlin_scheme_package(prefix: &std::path::Path) -> bool {
    prefix.join("lib/marlin-agent-core/init.ssi").is_file()
}

#[cfg(any(test, all(unix, not(feature = "linked-ui"))))]
fn resolve_local_scheme_prefix(
    current_gerbil_path: Option<&std::path::Path>,
    cwd: &std::path::Path,
    exe: Option<&std::path::Path>,
) -> Option<std::path::PathBuf> {
    if current_gerbil_path.is_some_and(has_marlin_scheme_package) {
        return None;
    }
    let mut candidates = vec![cwd.join("scheme")];
    if let Some(repo_root) = exe
        .and_then(|path| path.parent())
        .and_then(|profile_dir| profile_dir.parent())
        .and_then(|target_dir| target_dir.parent())
    {
        let from_exe = repo_root.join("scheme");
        if from_exe != candidates[0] {
            candidates.push(from_exe);
        }
    }
    for package in candidates {
        let prefix = package.join(".gerbil");
        if package.join("gerbil.pkg").is_file()
            && has_marlin_scheme_package(&prefix)
            && current_gerbil_path != Some(prefix.as_path())
        {
            return Some(prefix);
        }
    }
    None
}

#[cfg(all(unix, not(feature = "linked-ui")))]
fn use_local_scheme_package() -> std::io::Result<()> {
    use std::{env, os::unix::process::CommandExt, path::PathBuf, process::Command};

    let current_gerbil_path = env::var_os("GERBIL_PATH").map(PathBuf::from);
    let cwd = env::current_dir()?;
    let exe = env::current_exe().ok();
    let Some(prefix) =
        resolve_local_scheme_prefix(current_gerbil_path.as_deref(), &cwd, exe.as_deref())
    else {
        return Ok(());
    };
    // Replace the process before any VM or asynchronous runtime starts. This
    // gives Gerbil its normal package environment without mutating the host
    // process environment or introducing a second module loader.
    Err(Command::new(env::current_exe()?)
        .args(env::args_os().skip(1))
        .env("GERBIL_PATH", prefix)
        .exec())
}

/// Launch the terminal client after selecting its local Scheme package when needed.
pub fn launch() {
    #[cfg(all(unix, not(feature = "linked-ui")))]
    if let Err(error) = use_local_scheme_package() {
        eprintln!("marlin-tui: cannot start with the local Scheme package: {error}");
        std::process::exit(1);
    }
    crate::run_cli();
}

#[cfg(test)]
#[path = "../tests/unit/bootstrap.rs"]
mod tests;
