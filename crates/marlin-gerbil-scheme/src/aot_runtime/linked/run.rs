// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Runner for native Gerbil runtime AOT link-unit builds.

use super::{
    config::GerbilAotConfig,
    receipt::{
        GerbilAotBuildReceipt, GerbilAotCommandPlan, GerbilAotCommandReceipt, GerbilAotPlan,
        GerbilAotSymbol, GerbilAotSymbolAuditMethod,
    },
    status::{GerbilAotBuildStatus, GerbilAotStatus},
};
use object::{Object, ObjectSymbol};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Default)]
struct AotBuildCommandReceipts {
    gsc_compile_object: Option<GerbilAotCommandReceipt>,
    gsc_compile_dependency_objects: Vec<GerbilAotCommandReceipt>,
    gsc_generate_link_source: Option<GerbilAotCommandReceipt>,
    gsc_compile_link_object: Option<GerbilAotCommandReceipt>,
    symbol_audit_method: Option<GerbilAotSymbolAuditMethod>,
    symbol_audit: Option<GerbilAotCommandReceipt>,
}

struct DependencyObjectBuildFailure {
    status: GerbilAotBuildStatus,
    detail: Option<String>,
    receipts: Vec<GerbilAotCommandReceipt>,
}

pub(super) fn build_gerbil_aot_link_unit(config: &GerbilAotConfig) -> GerbilAotBuildReceipt {
    let initial_plan = config.plan();
    if initial_plan.status == GerbilAotStatus::MissingGsc {
        return aot_build_receipt(
            GerbilAotBuildStatus::MissingGsc,
            initial_plan,
            None,
            AotBuildCommandReceipts::default(),
            Vec::new(),
        );
    }
    if initial_plan.status == GerbilAotStatus::MissingCompiledRuntime {
        let detail = initial_plan.detail.clone();
        return aot_build_receipt(
            GerbilAotBuildStatus::MissingCompiledRuntime,
            initial_plan,
            detail,
            AotBuildCommandReceipts::default(),
            Vec::new(),
        );
    }

    let plan = config.plan();
    if let Some(status) = non_ready_plan_build_status(plan.status) {
        let detail = plan.detail.clone();
        return aot_build_receipt(
            status,
            plan,
            detail,
            AotBuildCommandReceipts::default(),
            Vec::new(),
        );
    }
    if let Err(error) = fs::create_dir_all(&plan.output_dir) {
        return aot_build_receipt(
            GerbilAotBuildStatus::OutputDirCreateFailed,
            plan,
            Some(error.to_string()),
            AotBuildCommandReceipts::default(),
            Vec::new(),
        );
    }

    let gsc_compile_dependency_objects = match compile_dependency_objects(&plan) {
        Ok(receipts) => receipts,
        Err(failure) => {
            return aot_build_receipt(
                failure.status,
                plan,
                failure.detail,
                AotBuildCommandReceipts {
                    gsc_compile_dependency_objects: failure.receipts,
                    ..Default::default()
                },
                Vec::new(),
            );
        }
    };

    let gsc_compile_object = run_aot_command(&plan, &plan.gsc_compile_object);
    if gsc_compile_object
        .status_code
        .is_none_or(|status| status != 0)
    {
        return aot_build_receipt(
            GerbilAotBuildStatus::GscCompileObjectFailed,
            plan,
            None,
            AotBuildCommandReceipts {
                gsc_compile_dependency_objects,
                gsc_compile_object: Some(gsc_compile_object),
                ..Default::default()
            },
            Vec::new(),
        );
    }
    if !plan.object.is_file() {
        let detail = Some(format!(
            "gsc completed without producing {}",
            plan.object.display()
        ));
        return aot_build_receipt(
            GerbilAotBuildStatus::ObjectMissing,
            plan,
            detail,
            AotBuildCommandReceipts {
                gsc_compile_dependency_objects,
                gsc_compile_object: Some(gsc_compile_object),
                ..Default::default()
            },
            Vec::new(),
        );
    }

    let gsc_generate_link_source = run_aot_command(&plan, &plan.gsc_generate_link_source);
    if gsc_generate_link_source
        .status_code
        .is_none_or(|status| status != 0)
    {
        return aot_build_receipt(
            GerbilAotBuildStatus::GscGenerateLinkSourceFailed,
            plan,
            None,
            AotBuildCommandReceipts {
                gsc_compile_dependency_objects,
                gsc_compile_object: Some(gsc_compile_object),
                gsc_generate_link_source: Some(gsc_generate_link_source),
                ..Default::default()
            },
            Vec::new(),
        );
    }
    if !plan.link_c_source.is_file() {
        let detail = Some(format!(
            "gsc -link completed without producing {}",
            plan.link_c_source.display()
        ));
        return aot_build_receipt(
            GerbilAotBuildStatus::LinkSourceMissing,
            plan,
            detail,
            AotBuildCommandReceipts {
                gsc_compile_dependency_objects,
                gsc_compile_object: Some(gsc_compile_object),
                gsc_generate_link_source: Some(gsc_generate_link_source),
                ..Default::default()
            },
            Vec::new(),
        );
    }

    let gsc_compile_link_object = run_aot_command(&plan, &plan.gsc_compile_link_object);
    if gsc_compile_link_object
        .status_code
        .is_none_or(|status| status != 0)
    {
        return aot_build_receipt(
            GerbilAotBuildStatus::GscCompileLinkObjectFailed,
            plan,
            None,
            AotBuildCommandReceipts {
                gsc_compile_dependency_objects,
                gsc_compile_object: Some(gsc_compile_object),
                gsc_generate_link_source: Some(gsc_generate_link_source),
                gsc_compile_link_object: Some(gsc_compile_link_object),
                ..Default::default()
            },
            Vec::new(),
        );
    }
    if !plan.link_object.is_file() {
        let detail = Some(format!(
            "gsc completed without producing {}",
            plan.link_object.display()
        ));
        return aot_build_receipt(
            GerbilAotBuildStatus::LinkObjectMissing,
            plan,
            detail,
            AotBuildCommandReceipts {
                gsc_compile_dependency_objects,
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
            let detail = Some(format!(
                "native object is missing required {} ABI symbols",
                plan.profile.label()
            ));
            return aot_build_receipt(
                GerbilAotBuildStatus::RequiredSymbolsMissing,
                plan,
                detail,
                AotBuildCommandReceipts {
                    gsc_compile_object: Some(gsc_compile_object),
                    gsc_generate_link_source: Some(gsc_generate_link_source),
                    gsc_compile_link_object: Some(gsc_compile_link_object),
                    symbol_audit_method: Some(GerbilAotSymbolAuditMethod::ObjectFiles),
                    ..Default::default()
                },
                missing_symbols,
            );
        }

        return aot_build_receipt(
            GerbilAotBuildStatus::LinkUnitReady,
            plan,
            None,
            AotBuildCommandReceipts {
                gsc_compile_dependency_objects,
                gsc_compile_object: Some(gsc_compile_object),
                gsc_generate_link_source: Some(gsc_generate_link_source),
                gsc_compile_link_object: Some(gsc_compile_link_object),
                symbol_audit_method: Some(GerbilAotSymbolAuditMethod::ObjectFiles),
                ..Default::default()
            },
            Vec::new(),
        );
    }

    let symbol_audit = run_aot_command(&plan, &plan.audit_symbols);
    if symbol_audit.status_code.is_none_or(|status| status != 0) {
        return aot_build_receipt(
            GerbilAotBuildStatus::SymbolAuditFailed,
            plan,
            None,
            AotBuildCommandReceipts {
                gsc_compile_dependency_objects,
                gsc_compile_object: Some(gsc_compile_object),
                gsc_generate_link_source: Some(gsc_generate_link_source),
                gsc_compile_link_object: Some(gsc_compile_link_object),
                symbol_audit_method: Some(GerbilAotSymbolAuditMethod::SymbolTableCommand),
                symbol_audit: Some(symbol_audit),
            },
            Vec::new(),
        );
    }
    let missing_symbols = missing_exported_symbols(&plan, &symbol_audit.stdout);
    if !missing_symbols.is_empty() {
        let detail = Some(format!(
            "native object is missing required {} ABI symbols",
            plan.profile.label()
        ));
        return aot_build_receipt(
            GerbilAotBuildStatus::RequiredSymbolsMissing,
            plan,
            detail,
            AotBuildCommandReceipts {
                gsc_compile_dependency_objects,
                gsc_compile_object: Some(gsc_compile_object),
                gsc_generate_link_source: Some(gsc_generate_link_source),
                gsc_compile_link_object: Some(gsc_compile_link_object),
                symbol_audit_method: Some(GerbilAotSymbolAuditMethod::SymbolTableCommand),
                symbol_audit: Some(symbol_audit),
            },
            missing_symbols,
        );
    }

    aot_build_receipt(
        GerbilAotBuildStatus::LinkUnitReady,
        plan,
        None,
        AotBuildCommandReceipts {
            gsc_compile_dependency_objects,
            gsc_compile_object: Some(gsc_compile_object),
            gsc_generate_link_source: Some(gsc_generate_link_source),
            gsc_compile_link_object: Some(gsc_compile_link_object),
            symbol_audit_method: Some(GerbilAotSymbolAuditMethod::SymbolTableCommand),
            symbol_audit: Some(symbol_audit),
        },
        Vec::new(),
    )
}

fn compile_dependency_objects(
    plan: &GerbilAotPlan,
) -> Result<Vec<GerbilAotCommandReceipt>, DependencyObjectBuildFailure> {
    plan.gsc_compile_dependency_objects
        .iter()
        .zip(&plan.dependency_objects)
        .try_fold(Vec::new(), |mut receipts, (command_plan, object)| {
            let receipt = run_aot_command(plan, command_plan);
            if receipt.status_code.is_none_or(|status| status != 0) {
                receipts.push(receipt);
                return Err(DependencyObjectBuildFailure {
                    status: GerbilAotBuildStatus::GscCompileDependencyObjectFailed,
                    detail: None,
                    receipts,
                });
            }
            receipts.push(receipt);

            if !object.is_file() {
                return Err(DependencyObjectBuildFailure {
                    status: GerbilAotBuildStatus::ObjectMissing,
                    detail: Some(format!(
                        "gsc completed without producing {}",
                        object.display()
                    )),
                    receipts,
                });
            }
            Ok(receipts)
        })
}

fn non_ready_plan_build_status(status: GerbilAotStatus) -> Option<GerbilAotBuildStatus> {
    match status {
        GerbilAotStatus::MissingGsc => Some(GerbilAotBuildStatus::MissingGsc),
        GerbilAotStatus::MissingCompiledRuntime => {
            Some(GerbilAotBuildStatus::MissingCompiledRuntime)
        }
        GerbilAotStatus::MissingHeader => Some(GerbilAotBuildStatus::MissingHeader),
        GerbilAotStatus::ReadyToBuildLinkUnit => None,
    }
}

fn run_aot_command(
    plan: &GerbilAotPlan,
    command_plan: &GerbilAotCommandPlan,
) -> GerbilAotCommandReceipt {
    let program = gerbil_toolchain_program(&command_plan.program);
    let mut command = Command::new(&program);
    command.current_dir(&plan.root).args(&command_plan.args);
    gerbil_scheme::tool_environment::configure_gerbil_native_tool_command(&mut command, &program);

    let output = command.output();

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

fn gerbil_toolchain_program(program: &Path) -> PathBuf {
    gerbil_scheme::GerbilToolchain::from_env().program(program)
}

fn missing_exported_symbols(plan: &GerbilAotPlan, symbol_table: &str) -> Vec<GerbilAotSymbol> {
    plan.exported_symbols
        .iter()
        .filter(|symbol| !symbol_table_contains(symbol_table, symbol.as_str()))
        .cloned()
        .collect()
}

fn missing_exported_symbols_from_object_files(
    plan: &GerbilAotPlan,
) -> Result<Vec<GerbilAotSymbol>, String> {
    let mut symbol_names = HashSet::new();
    for path in plan
        .module_objects
        .iter()
        .chain(std::iter::once(&plan.link_object))
    {
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

fn aot_build_receipt(
    status: GerbilAotBuildStatus,
    plan: GerbilAotPlan,
    detail: Option<String>,
    commands: AotBuildCommandReceipts,
    missing_symbols: Vec<GerbilAotSymbol>,
) -> GerbilAotBuildReceipt {
    GerbilAotBuildReceipt {
        status,
        plan,
        detail,
        gsc_compile_object: commands.gsc_compile_object,
        gsc_compile_dependency_objects: commands.gsc_compile_dependency_objects,
        gsc_generate_link_source: commands.gsc_generate_link_source,
        gsc_compile_link_object: commands.gsc_compile_link_object,
        symbol_audit_method: commands.symbol_audit_method,
        symbol_audit: commands.symbol_audit,
        missing_symbols,
    }
}
