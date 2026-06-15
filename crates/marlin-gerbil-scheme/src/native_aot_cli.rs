//! CLI entrypoint for building Rust-linkable native Gerbil AOT link units.

use std::{
    env,
    path::{Path, PathBuf},
    process::ExitCode,
};

use crate::{
    GerbilDeckRuntimeNativeAotBuildStatus, GerbilDeckRuntimeNativeAotConfig,
    GerbilDeckRuntimeNativeAotPlan, GerbilNativeLinkLibrary,
};

const DEFAULT_NATIVE_AOT_ROOT: &str = "target/marlin-gerbil-native-aot";
const NATIVE_AOT_GAMBIT_INCLUDE_ENV: &str = "MARLIN_GERBIL_NATIVE_AOT_GAMBIT_INCLUDE";

/// Runs the `marlin-gerbil-native-aot` command-line interface.
pub fn run_gerbil_native_aot_cli() -> ExitCode {
    match GerbilNativeAotCliConfig::from_args(env::args().skip(1)) {
        Ok(config) => run_gerbil_native_aot_action(config),
        Err(message) => {
            eprintln!("{message}");
            eprintln!("{}", GerbilNativeAotCliConfig::usage());
            ExitCode::from(2)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GerbilNativeAotCliConfig {
    action: GerbilNativeAotCliAction,
    root: PathBuf,
    output_dir: Option<PathBuf>,
    compiled_runtime_scm: Option<PathBuf>,
    gsc: Option<PathBuf>,
    c_compiler: Option<String>,
    symbol_auditor: Option<PathBuf>,
    gambit_link_library: Option<String>,
    gambit_link_search_dir: Option<PathBuf>,
    gambit_include_dir: Option<PathBuf>,
}

impl GerbilNativeAotCliConfig {
    fn from_args(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut args = args.into_iter();
        let action = match args.next().as_deref() {
            Some("build") => GerbilNativeAotCliAction::Build,
            Some("-h" | "--help") | None => GerbilNativeAotCliAction::Help,
            Some(action) => return Err(format!("unknown native AOT action `{action}`")),
        };
        let mut config = Self {
            action,
            root: default_native_aot_root(),
            output_dir: None,
            compiled_runtime_scm: None,
            gsc: None,
            c_compiler: default_native_aot_c_compiler(),
            symbol_auditor: None,
            gambit_link_library: None,
            gambit_link_search_dir: None,
            gambit_include_dir: None,
        };

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--root" => config.root = required_path_arg(&mut args, "--root")?,
                "--output-dir" => {
                    config.output_dir = Some(required_path_arg(&mut args, "--output-dir")?);
                }
                "--compiled-runtime-scm" => {
                    config.compiled_runtime_scm =
                        Some(required_path_arg(&mut args, "--compiled-runtime-scm")?);
                }
                "--gsc" => config.gsc = Some(required_path_arg(&mut args, "--gsc")?),
                "--c-compiler" => {
                    config.c_compiler = Some(required_string_arg(&mut args, "--c-compiler")?);
                }
                "--symbol-auditor" => {
                    config.symbol_auditor = Some(required_path_arg(&mut args, "--symbol-auditor")?);
                }
                "--gambit-link-lib" => {
                    config.gambit_link_library =
                        Some(required_string_arg(&mut args, "--gambit-link-lib")?);
                }
                "--gambit-link-search" => {
                    config.gambit_link_search_dir =
                        Some(required_path_arg(&mut args, "--gambit-link-search")?);
                }
                "--gambit-include" => {
                    config.gambit_include_dir =
                        Some(required_path_arg(&mut args, "--gambit-include")?);
                }
                "-h" | "--help" => config.action = GerbilNativeAotCliAction::Help,
                unknown => return Err(format!("unknown native AOT option `{unknown}`")),
            }
        }

        Ok(config)
    }

    fn usage() -> &'static str {
        "Usage: marlin-gerbil-native-aot <build> [--root <path>] [--output-dir <path>] [--compiled-runtime-scm <path>] [--gsc <path>] [--c-compiler <name>] [--symbol-auditor <path>] [--gambit-link-lib <name>] [--gambit-link-search <dir>] [--gambit-include <dir>]"
    }

    fn build_config(&self) -> GerbilDeckRuntimeNativeAotConfig {
        let mut config = GerbilDeckRuntimeNativeAotConfig::new(&self.root);
        if let Some(output_dir) = &self.output_dir {
            config = config.with_output_dir(output_dir);
        }
        if let Some(compiled_runtime_scm) = &self.compiled_runtime_scm {
            config = config.with_compiled_runtime_scm(compiled_runtime_scm);
        }
        if let Some(gsc) = &self.gsc {
            config = config.with_gsc(gsc);
        }
        if let Some(c_compiler) = &self.c_compiler {
            config = config.with_c_compiler(c_compiler);
        }
        if let Some(symbol_auditor) = &self.symbol_auditor {
            config = config.with_symbol_auditor(symbol_auditor);
        }
        if let Some(library) = &self.gambit_link_library {
            config = config.with_gambit_link_library(library);
        }
        if let Some(search_dir) = &self.gambit_link_search_dir {
            config = config.with_gambit_link_search_dir(search_dir);
        }
        config
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum GerbilNativeAotCliAction {
    Build,
    Help,
}

fn run_gerbil_native_aot_action(config: GerbilNativeAotCliConfig) -> ExitCode {
    match config.action {
        GerbilNativeAotCliAction::Help => {
            println!("{}", GerbilNativeAotCliConfig::usage());
            ExitCode::SUCCESS
        }
        GerbilNativeAotCliAction::Build => {
            let receipt = config.build_config().build_link_unit();
            if receipt.status != GerbilDeckRuntimeNativeAotBuildStatus::LinkUnitReady {
                eprintln!("native Gerbil AOT link unit build failed: {receipt:#?}");
                return ExitCode::FAILURE;
            }
            print_native_aot_link_metadata(&receipt.plan, config.gambit_include_dir.as_deref());
            ExitCode::SUCCESS
        }
    }
}

fn print_native_aot_link_metadata(
    plan: &GerbilDeckRuntimeNativeAotPlan,
    explicit_include_dir: Option<&Path>,
) {
    println!(
        "MARLIN_GERBIL_NATIVE_AOT_MODULE_OBJECT={}",
        plan.object.display()
    );
    println!(
        "MARLIN_GERBIL_NATIVE_AOT_LINK_SOURCE={}",
        plan.link_c_source.display()
    );
    println!(
        "MARLIN_GERBIL_NATIVE_AOT_LINK_OBJECT={}",
        plan.link_object.display()
    );
    println!(
        "MARLIN_GERBIL_NATIVE_AOT_GAMBIT_LINK_LIB={}",
        link_library_name(&plan.gambit_link_library)
    );
    if let Some(search_dir) = &plan.gambit_link_search_dir {
        println!(
            "MARLIN_GERBIL_NATIVE_AOT_GAMBIT_LINK_SEARCH={}",
            search_dir.display()
        );
    }
    let include_dir = explicit_include_dir.map(Path::to_path_buf).or_else(|| {
        plan.gambit_link_search_dir
            .as_deref()
            .and_then(infer_gambit_include_dir)
    });
    if let Some(include_dir) = include_dir {
        println!("{NATIVE_AOT_GAMBIT_INCLUDE_ENV}={}", include_dir.display());
    }
}

fn link_library_name(library: &GerbilNativeLinkLibrary) -> &str {
    library.as_str()
}

fn required_path_arg(
    args: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<PathBuf, String> {
    args.next()
        .map(resolve_cli_path)
        .ok_or_else(|| format!("{option} requires a value"))
}

fn required_string_arg(
    args: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn default_native_aot_root() -> PathBuf {
    env::current_dir()
        .unwrap_or_else(|_| Path::new(".").to_path_buf())
        .join(DEFAULT_NATIVE_AOT_ROOT)
}

fn resolve_cli_path(path: impl Into<PathBuf>) -> PathBuf {
    let path = path.into();
    if path.is_absolute() {
        return path;
    }
    env::current_dir()
        .unwrap_or_else(|_| Path::new(".").to_path_buf())
        .join(path)
}

fn infer_gambit_include_dir(search_dir: &Path) -> Option<PathBuf> {
    let include_dir = search_dir.parent()?.join("include");
    include_dir
        .join("gambit.h")
        .is_file()
        .then_some(include_dir)
}

fn default_native_aot_c_compiler() -> Option<String> {
    cfg!(target_os = "macos").then(|| "clang".to_string())
}
