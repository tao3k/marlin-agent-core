//! Runner for native Gerbil Deck runtime AOT link-unit builds.

use super::{
    config::GerbilDeckRuntimeNativeAotConfig,
    receipt::{
        GerbilDeckRuntimeNativeAotBuildReceipt, GerbilDeckRuntimeNativeAotCommandPlan,
        GerbilDeckRuntimeNativeAotCommandReceipt, GerbilDeckRuntimeNativeAotPlan,
        GerbilDeckRuntimeNativeSymbol, GerbilDeckRuntimeNativeSymbolAuditMethod,
    },
    status::{GerbilDeckRuntimeNativeAotBuildStatus, GerbilDeckRuntimeNativeAotStatus},
};
use crate::runtime::{GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath, write_gerbil_runtime_assets};
use object::{Object, ObjectSymbol};
use std::{
    collections::HashSet,
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const GAMBOPT_ENV: &str = "GAMBOPT";
const GERBIL_HOME_ENV: &str = "GERBIL_HOME";

#[derive(Default)]
struct NativeBuildCommandReceipts {
    gxc_generate_scheme: Option<GerbilDeckRuntimeNativeAotCommandReceipt>,
    gsc_compile_object: Option<GerbilDeckRuntimeNativeAotCommandReceipt>,
    gsc_generate_link_source: Option<GerbilDeckRuntimeNativeAotCommandReceipt>,
    gsc_compile_link_object: Option<GerbilDeckRuntimeNativeAotCommandReceipt>,
    symbol_audit_method: Option<GerbilDeckRuntimeNativeSymbolAuditMethod>,
    symbol_audit: Option<GerbilDeckRuntimeNativeAotCommandReceipt>,
}

pub(super) fn build_gerbil_deck_runtime_native_link_unit(
    config: &GerbilDeckRuntimeNativeAotConfig,
) -> GerbilDeckRuntimeNativeAotBuildReceipt {
    let initial_plan = config.plan();
    if initial_plan.status == GerbilDeckRuntimeNativeAotStatus::MissingGxc {
        return native_build_receipt(
            GerbilDeckRuntimeNativeAotBuildStatus::MissingGxc,
            initial_plan,
            None,
            NativeBuildCommandReceipts::default(),
            Vec::new(),
        );
    }
    if initial_plan.status == GerbilDeckRuntimeNativeAotStatus::MissingGsc {
        return native_build_receipt(
            GerbilDeckRuntimeNativeAotBuildStatus::MissingGsc,
            initial_plan,
            None,
            NativeBuildCommandReceipts::default(),
            Vec::new(),
        );
    }
    if let Err(error) = write_gerbil_runtime_assets(&initial_plan.root) {
        return native_build_receipt(
            GerbilDeckRuntimeNativeAotBuildStatus::AssetWriteFailed,
            initial_plan,
            Some(error.to_string()),
            NativeBuildCommandReceipts::default(),
            Vec::new(),
        );
    }

    let plan = config.plan();
    if let Some(status) = non_ready_plan_build_status(plan.status) {
        let detail = plan.detail.clone();
        return native_build_receipt(
            status,
            plan,
            detail,
            NativeBuildCommandReceipts::default(),
            Vec::new(),
        );
    }
    if let Err(error) = fs::create_dir_all(&plan.output_dir) {
        return native_build_receipt(
            GerbilDeckRuntimeNativeAotBuildStatus::OutputDirCreateFailed,
            plan,
            Some(error.to_string()),
            NativeBuildCommandReceipts::default(),
            Vec::new(),
        );
    }

    let gxc_generate_scheme = run_native_aot_command(&plan, &plan.gxc_generate_scheme);
    if gxc_generate_scheme
        .status_code
        .is_none_or(|status| status != 0)
    {
        return native_build_receipt(
            GerbilDeckRuntimeNativeAotBuildStatus::GxcGenerateSchemeFailed,
            plan,
            None,
            NativeBuildCommandReceipts {
                gxc_generate_scheme: Some(gxc_generate_scheme),
                ..Default::default()
            },
            Vec::new(),
        );
    }
    if !plan.generated_runtime_scm.is_file() {
        return native_build_receipt(
            GerbilDeckRuntimeNativeAotBuildStatus::GeneratedSchemeMissing,
            plan,
            Some("gxc completed without producing deck-runtime-native~0.scm".to_string()),
            NativeBuildCommandReceipts {
                gxc_generate_scheme: Some(gxc_generate_scheme),
                ..Default::default()
            },
            Vec::new(),
        );
    }

    let gsc_compile_object = run_native_aot_command(&plan, &plan.gsc_compile_object);
    if gsc_compile_object
        .status_code
        .is_none_or(|status| status != 0)
    {
        return native_build_receipt(
            GerbilDeckRuntimeNativeAotBuildStatus::GscCompileObjectFailed,
            plan,
            None,
            NativeBuildCommandReceipts {
                gxc_generate_scheme: Some(gxc_generate_scheme),
                gsc_compile_object: Some(gsc_compile_object),
                ..Default::default()
            },
            Vec::new(),
        );
    }
    if !plan.object.is_file() {
        return native_build_receipt(
            GerbilDeckRuntimeNativeAotBuildStatus::ObjectMissing,
            plan,
            Some("gsc completed without producing deck-runtime-native~0.o".to_string()),
            NativeBuildCommandReceipts {
                gxc_generate_scheme: Some(gxc_generate_scheme),
                gsc_compile_object: Some(gsc_compile_object),
                ..Default::default()
            },
            Vec::new(),
        );
    }

    let gsc_generate_link_source = run_native_aot_command(&plan, &plan.gsc_generate_link_source);
    if gsc_generate_link_source
        .status_code
        .is_none_or(|status| status != 0)
    {
        return native_build_receipt(
            GerbilDeckRuntimeNativeAotBuildStatus::GscGenerateLinkSourceFailed,
            plan,
            None,
            NativeBuildCommandReceipts {
                gxc_generate_scheme: Some(gxc_generate_scheme),
                gsc_compile_object: Some(gsc_compile_object),
                gsc_generate_link_source: Some(gsc_generate_link_source),
                ..Default::default()
            },
            Vec::new(),
        );
    }
    if !plan.link_c_source.is_file() {
        return native_build_receipt(
            GerbilDeckRuntimeNativeAotBuildStatus::LinkSourceMissing,
            plan,
            Some("gsc -link completed without producing deck-runtime-native~0_.c".to_string()),
            NativeBuildCommandReceipts {
                gxc_generate_scheme: Some(gxc_generate_scheme),
                gsc_compile_object: Some(gsc_compile_object),
                gsc_generate_link_source: Some(gsc_generate_link_source),
                ..Default::default()
            },
            Vec::new(),
        );
    }

    let gsc_compile_link_object = run_native_aot_command(&plan, &plan.gsc_compile_link_object);
    if gsc_compile_link_object
        .status_code
        .is_none_or(|status| status != 0)
    {
        return native_build_receipt(
            GerbilDeckRuntimeNativeAotBuildStatus::GscCompileLinkObjectFailed,
            plan,
            None,
            NativeBuildCommandReceipts {
                gxc_generate_scheme: Some(gxc_generate_scheme),
                gsc_compile_object: Some(gsc_compile_object),
                gsc_generate_link_source: Some(gsc_generate_link_source),
                gsc_compile_link_object: Some(gsc_compile_link_object),
                ..Default::default()
            },
            Vec::new(),
        );
    }
    if !plan.link_object.is_file() {
        return native_build_receipt(
            GerbilDeckRuntimeNativeAotBuildStatus::LinkObjectMissing,
            plan,
            Some("gsc completed without producing deck-runtime-native~0_.o".to_string()),
            NativeBuildCommandReceipts {
                gxc_generate_scheme: Some(gxc_generate_scheme),
                gsc_compile_object: Some(gsc_compile_object),
                gsc_generate_link_source: Some(gsc_generate_link_source),
                gsc_compile_link_object: Some(gsc_compile_link_object),
                ..Default::default()
            },
            Vec::new(),
        );
    }

    if let Ok(missing_symbols) = missing_exported_symbols_from_object_files(&plan) {
        if !missing_symbols.is_empty() {
            return native_build_receipt(
                GerbilDeckRuntimeNativeAotBuildStatus::RequiredSymbolsMissing,
                plan,
                Some("native object is missing required Deck runtime ABI symbols".to_string()),
                NativeBuildCommandReceipts {
                    gxc_generate_scheme: Some(gxc_generate_scheme),
                    gsc_compile_object: Some(gsc_compile_object),
                    gsc_generate_link_source: Some(gsc_generate_link_source),
                    gsc_compile_link_object: Some(gsc_compile_link_object),
                    symbol_audit_method: Some(
                        GerbilDeckRuntimeNativeSymbolAuditMethod::ObjectFiles,
                    ),
                    ..Default::default()
                },
                missing_symbols,
            );
        }

        return native_build_receipt(
            GerbilDeckRuntimeNativeAotBuildStatus::LinkUnitReady,
            plan,
            None,
            NativeBuildCommandReceipts {
                gxc_generate_scheme: Some(gxc_generate_scheme),
                gsc_compile_object: Some(gsc_compile_object),
                gsc_generate_link_source: Some(gsc_generate_link_source),
                gsc_compile_link_object: Some(gsc_compile_link_object),
                symbol_audit_method: Some(GerbilDeckRuntimeNativeSymbolAuditMethod::ObjectFiles),
                ..Default::default()
            },
            Vec::new(),
        );
    }

    let symbol_audit = run_native_aot_command(&plan, &plan.audit_symbols);
    if symbol_audit.status_code.is_none_or(|status| status != 0) {
        return native_build_receipt(
            GerbilDeckRuntimeNativeAotBuildStatus::SymbolAuditFailed,
            plan,
            None,
            NativeBuildCommandReceipts {
                gxc_generate_scheme: Some(gxc_generate_scheme),
                gsc_compile_object: Some(gsc_compile_object),
                gsc_generate_link_source: Some(gsc_generate_link_source),
                gsc_compile_link_object: Some(gsc_compile_link_object),
                symbol_audit_method: Some(
                    GerbilDeckRuntimeNativeSymbolAuditMethod::SymbolTableCommand,
                ),
                symbol_audit: Some(symbol_audit),
            },
            Vec::new(),
        );
    }
    let missing_symbols = missing_exported_symbols(&plan, &symbol_audit.stdout);
    if !missing_symbols.is_empty() {
        return native_build_receipt(
            GerbilDeckRuntimeNativeAotBuildStatus::RequiredSymbolsMissing,
            plan,
            Some("native object is missing required Deck runtime ABI symbols".to_string()),
            NativeBuildCommandReceipts {
                gxc_generate_scheme: Some(gxc_generate_scheme),
                gsc_compile_object: Some(gsc_compile_object),
                gsc_generate_link_source: Some(gsc_generate_link_source),
                gsc_compile_link_object: Some(gsc_compile_link_object),
                symbol_audit_method: Some(
                    GerbilDeckRuntimeNativeSymbolAuditMethod::SymbolTableCommand,
                ),
                symbol_audit: Some(symbol_audit),
            },
            missing_symbols,
        );
    }

    native_build_receipt(
        GerbilDeckRuntimeNativeAotBuildStatus::LinkUnitReady,
        plan,
        None,
        NativeBuildCommandReceipts {
            gxc_generate_scheme: Some(gxc_generate_scheme),
            gsc_compile_object: Some(gsc_compile_object),
            gsc_generate_link_source: Some(gsc_generate_link_source),
            gsc_compile_link_object: Some(gsc_compile_link_object),
            symbol_audit_method: Some(GerbilDeckRuntimeNativeSymbolAuditMethod::SymbolTableCommand),
            symbol_audit: Some(symbol_audit),
        },
        Vec::new(),
    )
}

fn non_ready_plan_build_status(
    status: GerbilDeckRuntimeNativeAotStatus,
) -> Option<GerbilDeckRuntimeNativeAotBuildStatus> {
    match status {
        GerbilDeckRuntimeNativeAotStatus::MissingGxc => {
            Some(GerbilDeckRuntimeNativeAotBuildStatus::MissingGxc)
        }
        GerbilDeckRuntimeNativeAotStatus::MissingGsc => {
            Some(GerbilDeckRuntimeNativeAotBuildStatus::MissingGsc)
        }
        GerbilDeckRuntimeNativeAotStatus::MissingSchemeSource => {
            Some(GerbilDeckRuntimeNativeAotBuildStatus::AssetWriteFailed)
        }
        GerbilDeckRuntimeNativeAotStatus::MissingHeader => {
            Some(GerbilDeckRuntimeNativeAotBuildStatus::MissingHeader)
        }
        GerbilDeckRuntimeNativeAotStatus::ReadyToBuildLinkUnit => None,
    }
}

fn run_native_aot_command(
    plan: &GerbilDeckRuntimeNativeAotPlan,
    command_plan: &GerbilDeckRuntimeNativeAotCommandPlan,
) -> GerbilDeckRuntimeNativeAotCommandReceipt {
    let mut command = Command::new(&command_plan.program);
    command
        .current_dir(&plan.root)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(&plan.root))
        .args(&command_plan.args);
    configure_native_tool_environment(&mut command, &command_plan.program);

    let output = command.output();

    match output {
        Ok(output) => GerbilDeckRuntimeNativeAotCommandReceipt {
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        },
        Err(error) => GerbilDeckRuntimeNativeAotCommandReceipt {
            status_code: None,
            stdout: String::new(),
            stderr: error.to_string(),
        },
    }
}

fn configure_native_tool_environment(command: &mut Command, program: &Path) {
    let Some(prefix) = infer_gerbil_tool_prefix(program) else {
        return;
    };

    command.env(GERBIL_HOME_ENV, &prefix);
    command.env(GAMBOPT_ENV, merged_gambopt(&prefix));
}

fn infer_gerbil_tool_prefix(program: &Path) -> Option<PathBuf> {
    let prefix = program.parent()?.parent()?;
    let bin = prefix.join("bin");
    let lib = prefix.join("lib");
    let include = prefix.join("include");

    if !bin.is_dir()
        || !lib.join("gerbil").is_dir()
        || !include.join("gambit.h").is_file()
        || !has_gambit_runtime_library(&lib)
    {
        return None;
    }

    Some(prefix.to_path_buf())
}

fn has_gambit_runtime_library(lib: &Path) -> bool {
    ["libgambit.a", "libgambit.dylib", "libgambit.so"]
        .iter()
        .any(|file_name| lib.join(file_name).is_file())
}

fn merged_gambopt(prefix: &Path) -> OsString {
    let mut value = OsString::from(format!(
        "~~bin={},~~lib={},~~include={}",
        prefix.join("bin").display(),
        prefix.join("lib").display(),
        prefix.join("include").display()
    ));

    if let Some(existing) = env::var_os(GAMBOPT_ENV).filter(|existing| !existing.is_empty()) {
        value.push(",");
        value.push(existing);
    }

    value
}

fn missing_exported_symbols(
    plan: &GerbilDeckRuntimeNativeAotPlan,
    symbol_table: &str,
) -> Vec<GerbilDeckRuntimeNativeSymbol> {
    plan.exported_symbols
        .iter()
        .filter(|symbol| !symbol_table_contains(symbol_table, symbol.as_str()))
        .cloned()
        .collect()
}

fn missing_exported_symbols_from_object_files(
    plan: &GerbilDeckRuntimeNativeAotPlan,
) -> Result<Vec<GerbilDeckRuntimeNativeSymbol>, String> {
    let mut symbol_names = HashSet::new();
    for path in [&plan.object, &plan.link_object] {
        for name in object_file_symbol_names(path)? {
            symbol_names.insert(name);
        }
    }

    Ok(plan
        .exported_symbols
        .iter()
        .filter(|symbol| {
            let prefixed_symbol = format!("_{}", symbol.as_str());
            !symbol_names.contains(symbol.as_str()) && !symbol_names.contains(&prefixed_symbol)
        })
        .cloned()
        .collect())
}

fn object_file_symbol_names(path: &Path) -> Result<Vec<String>, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let object = object::File::parse(bytes.as_slice()).map_err(|error| error.to_string())?;
    let mut names = Vec::new();
    names.extend(
        object
            .symbols()
            .filter_map(|symbol| symbol.name().ok().map(str::to_owned)),
    );
    names.extend(
        object
            .dynamic_symbols()
            .filter_map(|symbol| symbol.name().ok().map(str::to_owned)),
    );
    Ok(names)
}

fn symbol_table_contains(symbol_table: &str, symbol: &str) -> bool {
    let prefixed_symbol = format!("_{symbol}");
    symbol_table
        .lines()
        .any(|line| line.contains(symbol) || line.contains(&prefixed_symbol))
}

fn native_build_receipt(
    status: GerbilDeckRuntimeNativeAotBuildStatus,
    plan: GerbilDeckRuntimeNativeAotPlan,
    detail: Option<String>,
    commands: NativeBuildCommandReceipts,
    missing_symbols: Vec<GerbilDeckRuntimeNativeSymbol>,
) -> GerbilDeckRuntimeNativeAotBuildReceipt {
    GerbilDeckRuntimeNativeAotBuildReceipt {
        status,
        plan,
        detail,
        gxc_generate_scheme: commands.gxc_generate_scheme,
        gsc_compile_object: commands.gsc_compile_object,
        gsc_generate_link_source: commands.gsc_generate_link_source,
        gsc_compile_link_object: commands.gsc_compile_link_object,
        symbol_audit_method: commands.symbol_audit_method,
        symbol_audit: commands.symbol_audit,
        missing_symbols,
    }
}
