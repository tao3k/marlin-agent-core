// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Consume an Interface-selected compiled programme through Gerbil's package CLI.
use gerbil_scheme_aot_build::{run_native_process, source_workspace};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// A checked path through the package's published Module namespace.
pub struct ProgramSelection<'a>(&'a [&'a str]);

impl<'a> ProgramSelection<'a> {
    /// Accept Scheme identifiers, never executable expressions.
    /// # Errors
    /// Rejects empty paths and identifiers outside the Module naming grammar.
    pub fn new(path: &'a [&'a str]) -> Result<Self, String> {
        if path.is_empty()
            || path.iter().any(|part| {
                part.is_empty()
                    || !part
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
            })
        {
            return Err("invalid published program selection".into());
        }
        Ok(Self(path))
    }
}

/// Project a built SSI graph through the selected Module's Interface.
/// Cargo supplies its pinned ABI as compiled Gambit carriers.
/// # Errors
/// Refuses failed admission, missing products and duplicate ABI units.
pub fn project_scheme_program(
    package: &Path,
    selection: ProgramSelection<'_>,
    output: &Path,
) -> Result<PathBuf, String> {
    let package = fs::canonicalize(package).map_err(|error| error.to_string())?;
    fs::create_dir_all(output).map_err(|error| error.to_string())?;
    let flags = Command::new("pkg-config")
        .args(["--libs", "openssl", "zlib", "sqlite3"])
        .output()
        .map_err(|error| error.to_string())?;
    if !flags.status.success() {
        return Err(String::from_utf8_lossy(&flags.stderr).into_owned());
    }
    let expression = format!(
        r#"
      (import (only-in :clan/poo/object .get .call)
              (only-in :marlin-agent-core/init marlin-agent-core.init.exports))
      (let ((projection (.call marlin-agent-core.init.exports .project-build '("init"))))
        (.call projection .project-program
          (.get marlin-agent-core.init.exports {} program-entry)
          (getenv "MARLIN_PROGRAM_DIRECTORY")
          (list "-ld-options" (getenv "MARLIN_PROGRAM_LD_OPTIONS"))))
    "#,
        selection.0.join(" ")
    );
    let mut command = Command::new("gxpkg");
    command
        .current_dir(&package)
        .env("GERBIL_PATH", package.join(".gerbil"))
        .env("MARLIN_PROGRAM_DIRECTORY", output)
        .env(
            "MARLIN_PROGRAM_LD_OPTIONS",
            String::from_utf8(flags.stdout).map_err(|error| error.to_string())?,
        )
        .args(["env", "gxi", "-e", &expression]);
    run_native_process(
        &mut command,
        "project the Interface-selected compiled programme",
        true,
    )?;

    let selected = output.join("program.json");
    let mut plan: Value =
        serde_json::from_slice(&fs::read(&selected).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let modules = plan
        .get_mut("modules")
        .and_then(Value::as_array_mut)
        .ok_or("compiler programme has no module array")?;
    let sdk = source_workspace();
    let mut host = Vec::new();
    // Cargo's ABI carriers are independent of the Scheme business module graph.
    for unit in ["utf8", "runtime"] {
        let identity = format!("gerbil-scheme-rust/scheme/{unit}");
        if modules.iter().any(|row| row["module"] == identity) {
            return Err(format!(
                "Scheme module graph imports the Rust SDK: {identity}"
            ));
        }
        let source = sdk.join(format!("scheme/generated/{unit}.scm"));
        if !source.is_file() {
            return Err(format!("Cargo SDK carrier missing: {}", source.display()));
        }
        println!("cargo:rerun-if-changed={}", source.display());
        host.push(json!({"module": identity, "scm": source, "system": false}));
    }
    let position = modules
        .iter()
        .position(|row| {
            !row["module"]
                .as_str()
                .is_some_and(|id| id.starts_with("gerbil/runtime/"))
        })
        .unwrap_or(modules.len());
    modules.splice(position..position, host);
    let manifest = output.join("linked-program.json");
    fs::write(
        &manifest,
        serde_json::to_vec(&plan).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    for source in [
        "gerbil.pkg",
        "init.ss",
        "build.ss",
        ".gerbil/lib/tao3k/yuanxu/module-system/building/build-projection.ssi",
        ".gerbil/lib/tao3k/yuanxu/module-system/building/program.ssi",
        ".gerbil/lib/tao3k/yuanxu/module-system/building/program-activation.ssi",
        ".gerbil/lib/tao3k/yuanxu/module-system/building/program-metadata.ssi",
    ] {
        println!("cargo:rerun-if-changed={}", package.join(source).display());
    }
    Ok(manifest)
}
