//! Fixed harness gate for running real `gxi` integration tests.

use crate::MARLIN_GERBIL_GXI_ENV;
use std::{
    collections::BTreeMap,
    env,
    ffi::{OsStr, OsString},
    fmt,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

const GERBIL_HOME_ENV: &str = "GERBIL_HOME";
const DEFAULT_TEST_FILTER: &str = "command::real_gxi";

/// Runs the real `gxi` harness gate.
pub fn run_real_gxi_gate_cli() -> ExitCode {
    match run_real_gxi_gate_from_args(env::args_os()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Runs the real `gxi` harness gate from explicit CLI arguments.
pub fn run_real_gxi_gate_from_args(
    args: impl IntoIterator<Item = OsString>,
) -> Result<(), RealGxiGateError> {
    let request = RealGxiGateRequest::parse(args.into_iter())?;
    if request.help {
        println!("{}", usage(&request.program));
        return Ok(());
    }

    let command = RealGxiGateCommand::from_request(&request)?;
    if request.print_command {
        println!("{}", command.describe());
        return Ok(());
    }

    let status = command.to_command().status().map_err(|error| {
        RealGxiGateError::message(format!("failed to run real gxi gate: {error}"))
    })?;
    if !status.success() {
        return Err(RealGxiGateError::message(format!(
            "real gxi gate failed with status {status}"
        )));
    }

    Ok(())
}

/// Failure returned by the real `gxi` harness gate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RealGxiGateError {
    message: String,
}

impl RealGxiGateError {
    fn message(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RealGxiGateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RealGxiGateError {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RealGxiGateRequest {
    program: String,
    workspace_root: Option<PathBuf>,
    gerbil_home: Option<PathBuf>,
    gxi: Option<PathBuf>,
    cargo: Option<PathBuf>,
    filter: String,
    print_command: bool,
    help: bool,
}

impl RealGxiGateRequest {
    fn parse(mut args: impl Iterator<Item = OsString>) -> Result<Self, RealGxiGateError> {
        let program = args
            .next()
            .and_then(|program| program.into_string().ok())
            .unwrap_or_else(|| "marlin-gerbil-real-gxi-gate".to_string());
        let mut request = Self {
            program,
            workspace_root: None,
            gerbil_home: None,
            gxi: None,
            cargo: None,
            filter: DEFAULT_TEST_FILTER.to_string(),
            print_command: false,
            help: false,
        };

        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--workspace-root" => {
                    request.workspace_root = Some(next_path_arg(&mut args, "--workspace-root")?);
                }
                "--gerbil-home" => {
                    request.gerbil_home = Some(next_path_arg(&mut args, "--gerbil-home")?);
                }
                "--gxi" => {
                    request.gxi = Some(next_path_arg(&mut args, "--gxi")?);
                }
                "--cargo" => {
                    request.cargo = Some(next_path_arg(&mut args, "--cargo")?);
                }
                "--filter" => {
                    request.filter = next_string_arg(&mut args, "--filter")?;
                }
                "--print-command" => {
                    request.print_command = true;
                }
                "-h" | "--help" => {
                    request.help = true;
                }
                other => {
                    return Err(RealGxiGateError::message(format!(
                        "unexpected argument {other:?}\n\n{}",
                        usage(&request.program)
                    )));
                }
            }
        }

        Ok(request)
    }
}

/// Planned command for the real `gxi` harness gate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RealGxiGateCommand {
    cargo: PathBuf,
    workspace_root: PathBuf,
    args: Vec<OsString>,
    env: BTreeMap<OsString, OsString>,
}

impl RealGxiGateCommand {
    /// Builds a real `gxi` gate command from explicit CLI arguments.
    pub fn from_args(args: impl IntoIterator<Item = OsString>) -> Result<Self, RealGxiGateError> {
        let request = RealGxiGateRequest::parse(args.into_iter())?;
        if request.help {
            return Err(RealGxiGateError::message(
                "help output does not produce a real gxi gate command",
            ));
        }
        Self::from_request(&request)
    }

    fn from_request(request: &RealGxiGateRequest) -> Result<Self, RealGxiGateError> {
        let workspace_root_explicit = request.workspace_root.is_some();
        let workspace_root = request
            .workspace_root
            .clone()
            .unwrap_or_else(default_workspace_root);
        let default_gerbil_home = workspace_root.join(".data").join("gerbil").join("build");
        let default_gxi = default_gerbil_home.join("bin").join("gxi");
        let gxi = request
            .gxi
            .clone()
            .or_else(|| default_gxi.is_file().then(|| default_gxi.clone()))
            .or_else(|| {
                (!workspace_root_explicit)
                    .then(|| env::var_os(MARLIN_GERBIL_GXI_ENV).map(PathBuf::from))
                    .flatten()
            })
            .unwrap_or(default_gxi);
        let gerbil_home = request
            .gerbil_home
            .clone()
            .or_else(|| infer_gerbil_home_from_gxi(&gxi))
            .or_else(|| {
                default_gerbil_home
                    .is_dir()
                    .then(|| default_gerbil_home.clone())
            })
            .or_else(|| {
                (!workspace_root_explicit)
                    .then(|| env::var_os(GERBIL_HOME_ENV).map(PathBuf::from))
                    .flatten()
            })
            .unwrap_or(default_gerbil_home);

        if !gxi.is_file() {
            return Err(RealGxiGateError::message(format!(
                "missing gxi executable at {}; set {MARLIN_GERBIL_GXI_ENV} or pass --gxi",
                gxi.display()
            )));
        }
        if !gerbil_home.is_dir() {
            return Err(RealGxiGateError::message(format!(
                "missing Gerbil home at {}; set {GERBIL_HOME_ENV} or pass --gerbil-home",
                gerbil_home.display()
            )));
        }

        let cargo = request
            .cargo
            .clone()
            .or_else(|| env::var_os("CARGO").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("cargo"));
        let args = vec![
            "test".into(),
            "-p".into(),
            "marlin-gerbil-scheme".into(),
            "--locked".into(),
            "--test".into(),
            "unit_test".into(),
            request.filter.clone().into(),
            "--".into(),
            "--ignored".into(),
        ];
        let env = BTreeMap::from([
            (
                OsString::from(GERBIL_HOME_ENV),
                gerbil_home.into_os_string(),
            ),
            (OsString::from(MARLIN_GERBIL_GXI_ENV), gxi.into_os_string()),
        ]);

        Ok(Self {
            cargo,
            workspace_root,
            args,
            env,
        })
    }

    fn to_command(&self) -> Command {
        let mut command = Command::new(&self.cargo);
        command.current_dir(&self.workspace_root).args(&self.args);
        command.envs(&self.env);
        command
    }

    /// Returns the Cargo executable used by the gate.
    pub fn cargo(&self) -> &Path {
        &self.cargo
    }

    /// Returns the workspace root where the gate command runs.
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Returns the Cargo arguments used by the gate command.
    pub fn args(&self) -> &[OsString] {
        &self.args
    }

    /// Returns the environment variables injected into the gate command.
    pub fn env(&self) -> &BTreeMap<OsString, OsString> {
        &self.env
    }

    /// Renders the gate command for dry-run diagnostics.
    pub fn describe(&self) -> String {
        let env = self
            .env
            .iter()
            .map(|(key, value)| format!("{}={}", display_os(key), display_os(value)))
            .collect::<Vec<_>>()
            .join(" ");
        let args = self
            .args
            .iter()
            .map(|arg| display_os(arg.as_os_str()))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "cd {} && {env} {} {args}",
            self.workspace_root.display(),
            self.cargo.display()
        )
    }
}

fn next_path_arg(
    args: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> Result<PathBuf, RealGxiGateError> {
    Ok(PathBuf::from(next_string_arg(args, flag)?))
}

fn next_string_arg(
    args: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> Result<String, RealGxiGateError> {
    let Some(value) = args.next() else {
        return Err(RealGxiGateError::message(format!(
            "missing value for {flag}"
        )));
    };
    value.into_string().map_err(|value| {
        RealGxiGateError::message(format!("argument for {flag} is not UTF-8: {value:?}"))
    })
}

fn default_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate manifest dir should be under crates/marlin-gerbil-scheme")
        .to_path_buf()
}

fn infer_gerbil_home_from_gxi(gxi: &Path) -> Option<PathBuf> {
    let bin = gxi.parent()?;
    if bin.file_name() != Some(OsStr::new("bin")) {
        return None;
    }
    bin.parent().map(Path::to_path_buf)
}

fn display_os(value: &OsStr) -> String {
    value.to_string_lossy().into_owned()
}

fn usage(program: &str) -> String {
    format!(
        "usage: {program} [--workspace-root <path>] [--gerbil-home <path>] [--gxi <path>] \
[--cargo <path>] [--filter <test-filter>] [--print-command]\n\n\
Runs: cargo test -p marlin-gerbil-scheme --locked --test unit_test <filter> -- --ignored\n\
Defaults: filter={DEFAULT_TEST_FILTER}, gxi=.data/gerbil/build/bin/gxi"
    )
}
