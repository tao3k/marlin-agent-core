// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Entrypoint for the Marlin terminal client.

#[cfg(all(unix, not(feature = "linked-ui")))]
fn use_local_scheme_package() -> std::io::Result<()> {
    use std::{env, os::unix::process::CommandExt, process::Command};

    if env::var_os("GERBIL_PATH").is_some() {
        return Ok(());
    }
    let package = env::current_dir()?.join("scheme");
    let prefix = package.join(".gerbil");
    if !package.join("gerbil.pkg").is_file()
        || !prefix.join("lib/marlin-agent-core/init.ssi").is_file()
    {
        return Ok(());
    }
    // Replace the process before any VM or asynchronous runtime starts. This
    // gives Gerbil its normal package environment without mutating the host
    // process environment or introducing a second module loader.
    Err(Command::new(env::current_exe()?)
        .args(env::args_os().skip(1))
        .env("GERBIL_PATH", prefix)
        .exec())
}

fn main() {
    #[cfg(all(unix, not(feature = "linked-ui")))]
    if let Err(error) = use_local_scheme_package() {
        eprintln!("marlin-tui: cannot start with the local Scheme package: {error}");
        std::process::exit(1);
    }
    marlin_agent_tui::run_cli();
}
