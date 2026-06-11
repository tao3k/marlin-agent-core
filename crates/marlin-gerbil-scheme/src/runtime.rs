//! Crate-shipped `Gerbil` runtime assets for the `marlin` adapter loadpath.

use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{Command, Output},
};

/// Environment variable that overrides the `gxi` executable path.
pub const MARLIN_GERBIL_GXI_ENV: &str = "MARLIN_GERBIL_GXI";
/// Environment variable that overrides the `gxc` executable path.
pub const MARLIN_GERBIL_GXC_ENV: &str = "MARLIN_GERBIL_GXC";
/// Environment variable that identifies the paired `Gerbil` Gambit compiler.
pub const MARLIN_GERBIL_GSC_ENV: &str = "MARLIN_GERBIL_GSC";

/// Homebrew `gerbil-scheme` executable path used when no override is present.
pub const DEFAULT_GERBIL_GXI_PROGRAM: &str = "/opt/homebrew/opt/gerbil-scheme/bin/gxi";
/// Homebrew `gerbil-scheme` compiler path used when no override is present.
pub const DEFAULT_GERBIL_GXC_PROGRAM: &str = "/opt/homebrew/opt/gerbil-scheme/bin/gxc";
/// Homebrew `gerbil-scheme` Gambit compiler path used when no override is present.
pub const DEFAULT_GERBIL_GSC_PROGRAM: &str = "/opt/homebrew/opt/gerbil-scheme/bin/gsc";

/// Gerbil loadpath environment variable consumed by `gxi`.
pub const GERBIL_LOADPATH_ENV: &str = "GERBIL_LOADPATH";

/// Module entry point for the crate-shipped Marlin command adapter.
pub const GERBIL_ADAPTER_MODULE: &str = ":marlin/adapter";

/// Runtime source asset that can be written into a `gxi` loadpath root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilRuntimeAsset {
    pub path: &'static str,
    pub source: &'static str,
}

/// Source-file launcher that runs the `:marlin/adapter` command adapter.
pub const GERBIL_COMMAND_ADAPTER_SOURCE: &str =
    include_str!("../fixtures/gerbil/command-adapter.ss");
/// Source-file launcher that runs newline-delimited command adapter requests.
pub const GERBIL_COMMAND_ADAPTER_BATCH_SOURCE: &str =
    include_str!("../fixtures/gerbil/command-adapter-batch.ss");
/// Build script for compiling the crate-shipped Gerbil runtime assets.
pub const GERBIL_BUILD_SOURCE: &str = include_str!("../fixtures/gerbil/build.ss");
/// Standalone smoke source used to verify `Gerbil` module loading.
pub const GERBIL_SMOKE_SOURCE: &str = include_str!("../fixtures/gerbil/smoke.ss");
/// Library module that reads a compile request and emits a compile response.
pub const GERBIL_MARLIN_ADAPTER_SOURCE: &str = include_str!("../fixtures/gerbil/marlin/adapter.ss");
/// Reader-backed source parser for `marlin` smoke artifact forms.
pub const GERBIL_MARLIN_PARSER_SOURCE: &str = include_str!("../fixtures/gerbil/marlin/parser.ss");
/// Protocol binding constructors and JSON serializers for `marlin` artifacts.
pub const GERBIL_MARLIN_PROTOCOL_SOURCE: &str =
    include_str!("../fixtures/gerbil/marlin/protocol.ss");
/// JSON request decoder for the Rust-to-`Gerbil` command protocol.
pub const GERBIL_MARLIN_REQUEST_SOURCE: &str = include_str!("../fixtures/gerbil/marlin/request.ss");

/// Complete file manifest required under a `GERBIL_LOADPATH` root.
pub const GERBIL_RUNTIME_ASSETS: &[GerbilRuntimeAsset] = &[
    GerbilRuntimeAsset {
        path: "command-adapter.ss",
        source: GERBIL_COMMAND_ADAPTER_SOURCE,
    },
    GerbilRuntimeAsset {
        path: "command-adapter-batch.ss",
        source: GERBIL_COMMAND_ADAPTER_BATCH_SOURCE,
    },
    GerbilRuntimeAsset {
        path: "build.ss",
        source: GERBIL_BUILD_SOURCE,
    },
    GerbilRuntimeAsset {
        path: "smoke.ss",
        source: GERBIL_SMOKE_SOURCE,
    },
    GerbilRuntimeAsset {
        path: "marlin/adapter.ss",
        source: GERBIL_MARLIN_ADAPTER_SOURCE,
    },
    GerbilRuntimeAsset {
        path: "marlin/parser.ss",
        source: GERBIL_MARLIN_PARSER_SOURCE,
    },
    GerbilRuntimeAsset {
        path: "marlin/protocol.ss",
        source: GERBIL_MARLIN_PROTOCOL_SOURCE,
    },
    GerbilRuntimeAsset {
        path: "marlin/request.ss",
        source: GERBIL_MARLIN_REQUEST_SOURCE,
    },
];

const GERBIL_AOT_MODULE_SOURCES: &[&str] = &[
    "marlin/protocol.ss",
    "marlin/request.ss",
    "marlin/parser.ss",
    "marlin/adapter.ss",
];

const GERBIL_AOT_EXECUTABLE_NAME: &str = "command-adapter-aot";
const GERBIL_AOT_OUTPUT_DIR: &str = ".gerbil/lib";

/// Status reported by a `Gerbil` ahead-of-time compiler probe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GerbilAotProbeStatus {
    MissingGxc,
    MissingGsc,
    GscBackendUnavailable,
    AssetWriteFailed,
    ModuleCompileFailed,
    ExecutableCompileFailed,
    ExecutableReady,
}

/// Captured command result for a `Gerbil` ahead-of-time compiler step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilAotCommandReceipt {
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Structured result for probing the `Gerbil` ahead-of-time compiler toolchain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilAotProbeReceipt {
    pub status: GerbilAotProbeStatus,
    pub gxc: PathBuf,
    pub gsc: PathBuf,
    pub root: PathBuf,
    pub executable: PathBuf,
    pub detail: Option<String>,
    pub module_compile: Option<GerbilAotCommandReceipt>,
    pub executable_compile: Option<GerbilAotCommandReceipt>,
}

/// Configuration for a `Gerbil` ahead-of-time runtime compiler probe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilAotProbeConfig {
    root: PathBuf,
    gxc: PathBuf,
    gsc: PathBuf,
}

impl GerbilAotProbeConfig {
    /// Builds a probe rooted at a writable runtime asset directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            gxc: default_gerbil_gxc_program(),
            gsc: default_gerbil_gsc_program(),
        }
    }

    /// Overrides the `gxc` executable used by the probe.
    pub fn with_gxc(mut self, gxc: impl Into<PathBuf>) -> Self {
        self.gxc = gxc.into();
        self
    }

    /// Overrides the paired `Gerbil` Gambit compiler used by the probe.
    pub fn with_gsc(mut self, gsc: impl Into<PathBuf>) -> Self {
        self.gsc = gsc.into();
        self
    }

    /// Runs the probe and returns a typed receipt instead of panicking.
    pub fn probe(&self) -> GerbilAotProbeReceipt {
        run_gerbil_aot_probe(self)
    }
}

/// Returns the crate-owned `Gerbil` runtime asset manifest.
pub fn gerbil_runtime_assets() -> &'static [GerbilRuntimeAsset] {
    GERBIL_RUNTIME_ASSETS
}

/// Returns the configured `gxi` executable path without checking filesystem state.
pub fn default_gerbil_gxi_program() -> PathBuf {
    env::var_os(MARLIN_GERBIL_GXI_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GERBIL_GXI_PROGRAM))
}

/// Returns the configured `gxc` executable path without checking filesystem state.
pub fn default_gerbil_gxc_program() -> PathBuf {
    env::var_os(MARLIN_GERBIL_GXC_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GERBIL_GXC_PROGRAM))
}

/// Returns the configured `Gerbil` Gambit compiler path without checking filesystem state.
pub fn default_gerbil_gsc_program() -> PathBuf {
    env::var_os(MARLIN_GERBIL_GSC_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GERBIL_GSC_PROGRAM))
}

/// Writes the crate-owned `Gerbil` runtime assets under a loadpath root.
pub fn write_gerbil_runtime_assets(root: impl AsRef<Path>) -> io::Result<Vec<PathBuf>> {
    let root = root.as_ref();
    let mut written = Vec::with_capacity(GERBIL_RUNTIME_ASSETS.len());
    for asset in GERBIL_RUNTIME_ASSETS {
        let path = root.join(asset.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, asset.source)?;
        written.push(path);
    }
    Ok(written)
}

fn run_gerbil_aot_probe(config: &GerbilAotProbeConfig) -> GerbilAotProbeReceipt {
    let executable = config.root.join(GERBIL_AOT_EXECUTABLE_NAME);
    if !config.gxc.is_file() {
        return gerbil_aot_probe_receipt(
            config,
            executable,
            GerbilAotProbeStatus::MissingGxc,
            Some(format!(
                "missing gxc executable at {}",
                config.gxc.display()
            )),
            None,
            None,
        );
    }
    if !config.gsc.is_file() {
        return gerbil_aot_probe_receipt(
            config,
            executable,
            GerbilAotProbeStatus::MissingGsc,
            Some(format!(
                "missing Gerbil Gambit compiler at {}",
                config.gsc.display()
            )),
            None,
            None,
        );
    }
    if let Err(error) = write_gerbil_runtime_assets(&config.root) {
        return gerbil_aot_probe_receipt(
            config,
            executable,
            GerbilAotProbeStatus::AssetWriteFailed,
            Some(error.to_string()),
            None,
            None,
        );
    }
    if let Err(error) = fs::create_dir_all(gerbil_aot_output_dir(&config.root)) {
        return gerbil_aot_probe_receipt(
            config,
            executable,
            GerbilAotProbeStatus::AssetWriteFailed,
            Some(error.to_string()),
            None,
            None,
        );
    }

    let module_compile = run_gerbil_aot_module_compile(config);
    if module_compile.status_code.is_none_or(|status| status != 0) {
        let status = classify_gerbil_aot_module_failure(&module_compile);
        return gerbil_aot_probe_receipt(
            config,
            executable,
            status,
            None,
            Some(module_compile),
            None,
        );
    }

    let executable_compile = run_gerbil_aot_executable_compile(config, &executable);
    let executable_ready = executable_compile
        .status_code
        .is_some_and(|status| status == 0)
        && executable.is_file();
    let status = if executable_ready {
        GerbilAotProbeStatus::ExecutableReady
    } else {
        GerbilAotProbeStatus::ExecutableCompileFailed
    };
    gerbil_aot_probe_receipt(
        config,
        executable,
        status,
        None,
        Some(module_compile),
        Some(executable_compile),
    )
}

fn run_gerbil_aot_module_compile(config: &GerbilAotProbeConfig) -> GerbilAotCommandReceipt {
    let mut command = gerbil_aot_command(config);
    command
        .arg("-d")
        .arg(gerbil_aot_output_dir(&config.root))
        .arg("-O")
        .args(GERBIL_AOT_MODULE_SOURCES);
    gerbil_aot_command_receipt(command.output())
}

fn run_gerbil_aot_executable_compile(
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
        .arg("command-adapter.ss");
    gerbil_aot_command_receipt(command.output())
}

fn gerbil_aot_command(config: &GerbilAotProbeConfig) -> Command {
    let mut command = Command::new(&config.gxc);
    command
        .current_dir(&config.root)
        .env(GERBIL_LOADPATH_ENV, &config.root);
    if let Some(parent) = config.gsc.parent() {
        command.env("PATH", prepend_path(parent));
    }
    command
}

fn gerbil_aot_output_dir(root: &Path) -> PathBuf {
    root.join(GERBIL_AOT_OUTPUT_DIR)
}

fn classify_gerbil_aot_module_failure(receipt: &GerbilAotCommandReceipt) -> GerbilAotProbeStatus {
    let output = format!("{}\n{}", receipt.stdout, receipt.stderr);
    if output.contains("No such file or directory") && output.contains("/gsc") {
        GerbilAotProbeStatus::GscBackendUnavailable
    } else {
        GerbilAotProbeStatus::ModuleCompileFailed
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

fn gerbil_aot_probe_receipt(
    config: &GerbilAotProbeConfig,
    executable: PathBuf,
    status: GerbilAotProbeStatus,
    detail: Option<String>,
    module_compile: Option<GerbilAotCommandReceipt>,
    executable_compile: Option<GerbilAotCommandReceipt>,
) -> GerbilAotProbeReceipt {
    GerbilAotProbeReceipt {
        status,
        gxc: config.gxc.clone(),
        gsc: config.gsc.clone(),
        root: config.root.clone(),
        executable,
        detail,
        module_compile,
        executable_compile,
    }
}
