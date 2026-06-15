//! Command execution helpers for Gerbil AOT compilation.

use super::{
    config::GerbilAotProbeConfig,
    constants::{GERBIL_AOT_MODULE_SOURCES, GERBIL_AOT_OUTPUT_DIR},
    receipt::{GerbilAotCommandReceipt, GerbilAotProbeReceipt},
    status::GerbilAotProbeStatus,
};
use crate::runtime::{GERBIL_LOADPATH_ENV, GERBIL_MARLIN_ADAPTER_PATH, gerbil_runtime_loadpath};
use std::{
    env, io,
    path::{Path, PathBuf},
    process::{Command, Output},
};

pub(super) fn run_gerbil_aot_module_compile(
    config: &GerbilAotProbeConfig,
) -> GerbilAotCommandReceipt {
    let mut command = gerbil_aot_command(config);
    command
        .arg("-d")
        .arg(gerbil_aot_output_dir(&config.root))
        .arg("-O")
        .args(GERBIL_AOT_MODULE_SOURCES);
    gerbil_aot_command_receipt(command.output())
}

pub(super) fn run_gerbil_aot_executable_compile(
    config: &GerbilAotProbeConfig,
    executable: &Path,
) -> GerbilAotCommandReceipt {
    let mut command = gerbil_aot_command(config);
    command
        .arg("-d")
        .arg(gerbil_aot_output_dir(&config.root))
        .arg("-exe")
        .arg("-O")
        .arg("-o")
        .arg(executable)
        .arg(GERBIL_MARLIN_ADAPTER_PATH);
    gerbil_aot_command_receipt(command.output())
}

pub(super) fn gerbil_aot_output_dir(root: &Path) -> PathBuf {
    root.join(GERBIL_AOT_OUTPUT_DIR)
}

pub(super) fn classify_gerbil_aot_module_failure(
    receipt: &GerbilAotCommandReceipt,
) -> GerbilAotProbeStatus {
    let output = format!("{}\n{}", receipt.stdout, receipt.stderr);
    if output.contains("No such file or directory") && output.contains("/gsc") {
        GerbilAotProbeStatus::GscBackendUnavailable
    } else {
        GerbilAotProbeStatus::ModuleCompileFailed
    }
}

pub(super) fn gerbil_aot_probe_receipt(
    config: &GerbilAotProbeConfig,
    executable: PathBuf,
    status: GerbilAotProbeStatus,
    detail: Option<String>,
    module_compile: Option<GerbilAotCommandReceipt>,
    executable_compile: Option<GerbilAotCommandReceipt>,
) -> GerbilAotProbeReceipt {
    let backend_gsc = if status == GerbilAotProbeStatus::GscBackendUnavailable {
        module_compile
            .as_ref()
            .and_then(gerbil_aot_backend_gsc_path)
    } else {
        None
    };
    GerbilAotProbeReceipt {
        status,
        gxc: config.gxc.clone(),
        gsc: config.gsc.clone(),
        backend_gsc,
        root: config.root.clone(),
        executable,
        detail,
        module_compile,
        executable_compile,
    }
}

fn gerbil_aot_command(config: &GerbilAotProbeConfig) -> Command {
    let mut command = Command::new(&config.gxc);
    command
        .current_dir(&config.root)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(&config.root));
    if let Some(parent) = config.gsc.parent() {
        command.env("PATH", prepend_path(parent));
    }
    command
}

fn gerbil_aot_backend_gsc_path(receipt: &GerbilAotCommandReceipt) -> Option<PathBuf> {
    let output = format!("{}\n{}", receipt.stdout, receipt.stderr);
    extract_quoted_gsc_path(&output).or_else(|| extract_unquoted_gsc_path(&output))
}

fn extract_quoted_gsc_path(output: &str) -> Option<PathBuf> {
    output
        .split('"')
        .skip(1)
        .step_by(2)
        .find(|quoted| quoted.ends_with("/gsc") || quoted.ends_with("\\gsc"))
        .map(PathBuf::from)
}

fn extract_unquoted_gsc_path(output: &str) -> Option<PathBuf> {
    output
        .split_whitespace()
        .filter_map(candidate_gsc_path_token)
        .find(|path| path.ends_with("gsc"))
}

fn candidate_gsc_path_token(token: &str) -> Option<PathBuf> {
    let trimmed = token.trim_matches(|character| {
        matches!(
            character,
            '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';'
        )
    });
    let candidate = trimmed
        .rsplit_once("path:")
        .map_or(trimmed, |(_, value)| value)
        .trim_matches(|character| matches!(character, '"' | '\'' | '(' | ')' | ',' | ';'));

    if candidate.ends_with("/gsc") || candidate.ends_with("\\gsc") {
        Some(PathBuf::from(candidate))
    } else {
        None
    }
}

fn prepend_path(path: &Path) -> std::ffi::OsString {
    match env::var_os("PATH") {
        Some(current) => {
            let mut paths = Vec::from([path.to_path_buf()]);
            paths.extend(env::split_paths(&current));
            env::join_paths(paths).unwrap_or(current)
        }
        None => path.as_os_str().to_os_string(),
    }
}

fn gerbil_aot_command_receipt(output: io::Result<Output>) -> GerbilAotCommandReceipt {
    match output {
        Ok(output) => GerbilAotCommandReceipt {
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        },
        Err(error) => GerbilAotCommandReceipt {
            status_code: None,
            stdout: String::new(),
            stderr: error.to_string(),
        },
    }
}
