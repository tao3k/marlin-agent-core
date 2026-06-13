//! Runner for native Gerbil Deck runtime AOT link-unit builds.

use super::{
    config::GerbilDeckRuntimeNativeAotConfig,
    receipt::{
        GerbilDeckRuntimeNativeAotBuildReceipt, GerbilDeckRuntimeNativeAotCommandPlan,
        GerbilDeckRuntimeNativeAotCommandReceipt, GerbilDeckRuntimeNativeAotPlan,
    },
    status::{GerbilDeckRuntimeNativeAotBuildStatus, GerbilDeckRuntimeNativeAotStatus},
};
use crate::runtime::{GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath, write_gerbil_runtime_assets};
use std::{fs, process::Command};

#[derive(Default)]
struct NativeBuildCommandReceipts {
    gxc_generate_scheme: Option<GerbilDeckRuntimeNativeAotCommandReceipt>,
    gsc_compile_object: Option<GerbilDeckRuntimeNativeAotCommandReceipt>,
    gsc_generate_link_source: Option<GerbilDeckRuntimeNativeAotCommandReceipt>,
    gsc_compile_link_object: Option<GerbilDeckRuntimeNativeAotCommandReceipt>,
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
    let output = Command::new(&command_plan.program)
        .current_dir(&plan.root)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(&plan.root))
        .args(&command_plan.args)
        .output();

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

fn missing_exported_symbols(
    plan: &GerbilDeckRuntimeNativeAotPlan,
    symbol_table: &str,
) -> Vec<super::receipt::GerbilDeckRuntimeNativeSymbol> {
    plan.exported_symbols
        .iter()
        .filter(|symbol| !symbol_table_contains(symbol_table, symbol.as_str()))
        .cloned()
        .collect()
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
    missing_symbols: Vec<super::receipt::GerbilDeckRuntimeNativeSymbol>,
) -> GerbilDeckRuntimeNativeAotBuildReceipt {
    GerbilDeckRuntimeNativeAotBuildReceipt {
        status,
        plan,
        detail,
        gxc_generate_scheme: commands.gxc_generate_scheme,
        gsc_compile_object: commands.gsc_compile_object,
        gsc_generate_link_source: commands.gsc_generate_link_source,
        gsc_compile_link_object: commands.gsc_compile_link_object,
        symbol_audit: commands.symbol_audit,
        missing_symbols,
    }
}
