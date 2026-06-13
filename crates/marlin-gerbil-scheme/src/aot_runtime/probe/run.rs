//! Probe runner for Gerbil AOT compiler readiness.

use super::{
    command::{
        classify_gerbil_aot_module_failure, gerbil_aot_output_dir, gerbil_aot_probe_receipt,
        run_gerbil_aot_executable_compile, run_gerbil_aot_module_compile,
    },
    config::GerbilAotProbeConfig,
    constants::GERBIL_AOT_EXECUTABLE_NAME,
    receipt::GerbilAotProbeReceipt,
    status::GerbilAotProbeStatus,
};
use crate::runtime::write_gerbil_runtime_assets;
use std::fs;

pub(super) fn run_gerbil_aot_probe(config: &GerbilAotProbeConfig) -> GerbilAotProbeReceipt {
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
