//! CLI entrypoint and argument parsing for `marlin-gerbil-deps`.

use super::{
    GERBIL_BIN_ENV, GERBIL_CELLAR_ENV, GERBIL_GCC_ENV, GERBIL_GSC_ENV, GERBIL_MACOS_SDK_ENV,
    GERBIL_PREFIX_ENV, GerbilDepsAction, GerbilDepsConfig, GerbilDepsError,
    MARLIN_GERBIL_PKG_CACHE_ENV,
};
use std::{env, ffi::OsString, path::PathBuf, process::ExitCode};

/// Runs the Gerbil dependency bootstrap CLI.
pub fn run_gerbil_deps_cli() -> ExitCode {
    match run_gerbil_deps_from_args(env::args_os()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Runs Gerbil dependency bootstrap from explicit CLI arguments.
pub fn run_gerbil_deps_from_args(
    args: impl IntoIterator<Item = OsString>,
) -> Result<(), GerbilDepsError> {
    let request = GerbilDepsRequest::parse(args.into_iter())?;
    if request.help {
        println!("{}", usage(&request.program));
        return Ok(());
    }

    let config = GerbilDepsConfig::from_request(&request)?;
    if request.print_plan {
        println!("{}", config.describe());
        return Ok(());
    }

    match request.action {
        GerbilDepsAction::Env => {
            println!("{}", config.describe());
            Ok(())
        }
        GerbilDepsAction::Repair => config.repair_homebrew_layout(),
        GerbilDepsAction::Fetch => config.fetch_packages(),
        GerbilDepsAction::Link => {
            config.repair_homebrew_layout()?;
            config.fetch_packages()?;
            config.link_packages()
        }
        GerbilDepsAction::Build => {
            config.repair_homebrew_layout()?;
            config.fetch_packages()?;
            config.link_packages()?;
            config.build_packages()
        }
        GerbilDepsAction::Verify => config.verify_packages(),
        GerbilDepsAction::Bootstrap => {
            config.repair_homebrew_layout()?;
            config.fetch_packages()?;
            config.link_packages()?;
            config.build_packages()?;
            config.verify_packages()
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct GerbilDepsRequest {
    pub(super) program: String,
    pub(super) action: GerbilDepsAction,
    pub(super) platform: Option<String>,
    pub(super) cache_dir: Option<PathBuf>,
    pub(super) gerbil_bin: Option<PathBuf>,
    pub(super) gerbil_cellar: Option<PathBuf>,
    pub(super) gerbil_prefix: Option<PathBuf>,
    pub(super) gerbil_gsc: Option<PathBuf>,
    pub(super) gerbil_gcc: Option<PathBuf>,
    pub(super) macos_sdk: Option<PathBuf>,
    pub(super) print_plan: bool,
    pub(super) help: bool,
}

impl GerbilDepsRequest {
    fn parse(mut args: impl Iterator<Item = OsString>) -> Result<Self, GerbilDepsError> {
        let program = args
            .next()
            .and_then(|program| program.into_string().ok())
            .unwrap_or_else(|| "marlin-gerbil-deps".to_string());
        let mut request = Self {
            program,
            action: GerbilDepsAction::Bootstrap,
            platform: None,
            cache_dir: None,
            gerbil_bin: None,
            gerbil_cellar: None,
            gerbil_prefix: None,
            gerbil_gsc: None,
            gerbil_gcc: None,
            macos_sdk: None,
            print_plan: false,
            help: false,
        };
        let mut action_seen = false;

        while let Some(arg) = args.next() {
            let arg_text = arg.to_string_lossy();
            match arg_text.as_ref() {
                "-h" | "--help" => request.help = true,
                "--print-plan" => request.print_plan = true,
                "--platform" => request.platform = Some(next_string_arg(&mut args, "--platform")?),
                "--cache-dir" => {
                    request.cache_dir = Some(next_path_arg(&mut args, "--cache-dir")?);
                }
                "--gerbil-bin" => {
                    request.gerbil_bin = Some(next_path_arg(&mut args, "--gerbil-bin")?);
                }
                "--gerbil-cellar" => {
                    request.gerbil_cellar = Some(next_path_arg(&mut args, "--gerbil-cellar")?);
                }
                "--gerbil-prefix" => {
                    request.gerbil_prefix = Some(next_path_arg(&mut args, "--gerbil-prefix")?);
                }
                "--gerbil-gsc" => {
                    request.gerbil_gsc = Some(next_path_arg(&mut args, "--gerbil-gsc")?);
                }
                "--gerbil-gcc" => {
                    request.gerbil_gcc = Some(next_path_arg(&mut args, "--gerbil-gcc")?);
                }
                "--macos-sdk" => {
                    request.macos_sdk = Some(next_path_arg(&mut args, "--macos-sdk")?);
                }
                other if other.starts_with('-') => {
                    return Err(GerbilDepsError::message(format!(
                        "unexpected argument {other:?}\n\n{}",
                        usage(&request.program)
                    )));
                }
                other => {
                    if action_seen {
                        return Err(GerbilDepsError::message(format!(
                            "unexpected positional argument {other:?}\n\n{}",
                            usage(&request.program)
                        )));
                    }
                    request.action = GerbilDepsAction::parse(other).ok_or_else(|| {
                        GerbilDepsError::message(format!(
                            "unknown Gerbil dependency action {other:?}\n\n{}",
                            usage(&request.program)
                        ))
                    })?;
                    action_seen = true;
                }
            }
        }

        Ok(request)
    }
}

fn next_path_arg(
    args: &mut impl Iterator<Item = OsString>,
    option: &str,
) -> Result<PathBuf, GerbilDepsError> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| GerbilDepsError::message(format!("missing value for {option}")))
}

fn next_string_arg(
    args: &mut impl Iterator<Item = OsString>,
    option: &str,
) -> Result<String, GerbilDepsError> {
    args.next()
        .and_then(|arg| arg.into_string().ok())
        .ok_or_else(|| GerbilDepsError::message(format!("missing UTF-8 value for {option}")))
}

fn usage(program: &str) -> String {
    format!(
        "Usage: {program} <env|repair|fetch|link|build|verify|bootstrap> [options]\n\n\
         Options:\n\
           --platform <name>        Override detected platform, e.g. macos or linux\n\
           --cache-dir <path>       Package checkout cache [env {MARLIN_GERBIL_PKG_CACHE_ENV}]\n\
           --gerbil-bin <path>      Directory containing gxi and gxpkg [env {GERBIL_BIN_ENV}]\n\
           --gerbil-cellar <path>   Homebrew Gerbil cellar root [env {GERBIL_CELLAR_ENV}]\n\
           --gerbil-prefix <path>   Gerbil runtime prefix [env {GERBIL_PREFIX_ENV}]\n\
           --gerbil-gsc <path>      Gambit compiler used by Gerbil [env {GERBIL_GSC_ENV}]\n\
           --gerbil-gcc <path>      C compiler used by Gerbil linking [env {GERBIL_GCC_ENV}]\n\
           --macos-sdk <path>       macOS SDK with _bounds.h and libz.tbd [env {GERBIL_MACOS_SDK_ENV}]\n\
           --print-plan             Print resolved config without executing\n\
           -h, --help               Print this help"
    )
}
