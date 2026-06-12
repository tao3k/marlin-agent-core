//! Command-line entrypoint for explicit Gerbil AOT backend repair.

use crate::{GerbilAotProbeConfig, GerbilAotProbeReceipt};
use std::{
    env,
    ffi::OsString,
    fmt,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

/// Runs the Gerbil AOT backend repair command.
pub fn run_gerbil_aot_repair_cli() -> ExitCode {
    let mut stdout = io::stdout();
    match run_gerbil_aot_repair_from_args(env::args_os().collect(), &mut stdout) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run_gerbil_aot_repair_from_args(
    args: Vec<OsString>,
    output: &mut dyn Write,
) -> Result<(), GerbilAotRepairCliError> {
    let request = GerbilAotRepairRequest::parse(args.into_iter())?;
    if request.help {
        writeln!(output, "{}", usage(&request.program))?;
        return Ok(());
    }

    let receipt = probe_for_request(&request)?;
    let plan = receipt
        .plan_backend_gsc_repair(&request.allowed_root)
        .map_err(GerbilAotRepairCliError::from_display)?;
    writeln!(
        output,
        "probe_status={:?} backend_repair={:?} gsc={} backend_gsc={} allowed_root={} \
can_create_shim={} requires_system_write={} recommended_action={}",
        receipt.status,
        plan.status,
        plan.gsc.display(),
        display_optional_path(&plan.backend_gsc),
        plan.allowed_root.display(),
        plan.can_create_shim,
        plan.requires_system_write,
        plan.recommended_action,
    )?;

    if !request.apply {
        return Ok(());
    }
    if plan.requires_system_write && !request.allow_system_write {
        return Err(GerbilAotRepairCliError::message(
            "refusing to write outside --allowed-root without --allow-system-write",
        ));
    }

    let allowed_root = if plan.requires_system_write {
        system_write_root()?
    } else {
        request.allowed_root
    };
    let shim = receipt
        .prepare_backend_gsc_shim(&allowed_root)
        .map_err(GerbilAotRepairCliError::from_display)?;
    writeln!(
        output,
        "backend_shim={:?} gsc={} backend_gsc={} allowed_root={}",
        shim.status,
        shim.gsc.display(),
        display_optional_path(&shim.backend_gsc),
        shim.allowed_root.display(),
    )?;

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GerbilAotRepairCliError {
    message: String,
}

impl GerbilAotRepairCliError {
    fn message(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn from_display(error: impl fmt::Display) -> Self {
        Self::message(error.to_string())
    }
}

impl fmt::Display for GerbilAotRepairCliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for GerbilAotRepairCliError {}

impl From<io::Error> for GerbilAotRepairCliError {
    fn from(error: io::Error) -> Self {
        Self::from_display(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GerbilAotRepairRequest {
    program: String,
    root: PathBuf,
    cache: Option<PathBuf>,
    gxc: Option<PathBuf>,
    gsc: Option<PathBuf>,
    allowed_root: PathBuf,
    apply: bool,
    allow_system_write: bool,
    help: bool,
}

impl GerbilAotRepairRequest {
    fn parse(mut args: impl Iterator<Item = OsString>) -> Result<Self, GerbilAotRepairCliError> {
        let program = args
            .next()
            .and_then(|program| program.into_string().ok())
            .unwrap_or_else(|| "marlin-gerbil-aot-repair".to_owned());
        let mut request = Self {
            program,
            root: default_probe_root(),
            cache: None,
            gxc: None,
            gsc: None,
            allowed_root: default_allowed_root(),
            apply: false,
            allow_system_write: false,
            help: false,
        };

        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--root" => {
                    request.root = next_path_arg(&mut args, "--root")?;
                }
                "--cache" => {
                    request.cache = Some(next_path_arg(&mut args, "--cache")?);
                }
                "--gxc" => {
                    request.gxc = Some(next_path_arg(&mut args, "--gxc")?);
                }
                "--gsc" => {
                    request.gsc = Some(next_path_arg(&mut args, "--gsc")?);
                }
                "--allowed-root" => {
                    request.allowed_root = next_path_arg(&mut args, "--allowed-root")?;
                }
                "--apply" => {
                    request.apply = true;
                }
                "--allow-system-write" => {
                    request.allow_system_write = true;
                }
                "-h" | "--help" => {
                    request.help = true;
                }
                other => {
                    return Err(GerbilAotRepairCliError::message(format!(
                        "unexpected argument {other:?}\n\n{}",
                        usage(&request.program)
                    )));
                }
            }
        }

        Ok(request)
    }
}

fn probe_for_request(
    request: &GerbilAotRepairRequest,
) -> Result<GerbilAotProbeReceipt, GerbilAotRepairCliError> {
    let mut config = GerbilAotProbeConfig::new(&request.root);
    if let Some(gxc) = request.gxc.clone() {
        config = config.with_gxc(gxc);
    }
    if let Some(gsc) = request.gsc.clone() {
        config = config.with_gsc(gsc);
    }

    match &request.cache {
        Some(cache) => config
            .probe_with_toolchain_cache(cache)
            .map_err(GerbilAotRepairCliError::from_display),
        None => Ok(config.probe()),
    }
}

fn next_path_arg(
    args: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> Result<PathBuf, GerbilAotRepairCliError> {
    Ok(PathBuf::from(next_string_arg(args, flag)?))
}

fn next_string_arg(
    args: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> Result<String, GerbilAotRepairCliError> {
    let Some(value) = args.next() else {
        return Err(GerbilAotRepairCliError::message(format!(
            "missing value for {flag}"
        )));
    };
    value.into_string().map_err(|value| {
        GerbilAotRepairCliError::message(format!("argument for {flag} is not UTF-8: {value:?}"))
    })
}

fn default_probe_root() -> PathBuf {
    env::temp_dir().join(format!("marlin-gerbil-aot-repair-{}", std::process::id()))
}

fn default_allowed_root() -> PathBuf {
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(unix)]
fn system_write_root() -> Result<PathBuf, GerbilAotRepairCliError> {
    Ok(PathBuf::from("/"))
}

#[cfg(not(unix))]
fn system_write_root() -> Result<PathBuf, GerbilAotRepairCliError> {
    Err(GerbilAotRepairCliError::message(
        "--allow-system-write is only implemented for Unix-style absolute paths",
    ))
}

fn display_optional_path(path: &Option<PathBuf>) -> String {
    path.as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "-".to_owned())
}

fn usage(program: &str) -> String {
    format!(
        "usage: {program} [--root <path>] [--cache <path>] [--gxc <path>] [--gsc <path>] \
[--allowed-root <path>] [--apply] [--allow-system-write]\n\n\
Probes Gerbil AOT compilation and reports the missing backend gsc repair plan. \
Default mode is dry-run only. --apply writes only inside --allowed-root unless \
--allow-system-write is also supplied."
    )
}
