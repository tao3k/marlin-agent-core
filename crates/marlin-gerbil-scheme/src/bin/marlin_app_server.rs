// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Desktop & Mobile Authority Host (`marlin-app-server`) projecting live
//! Gerbil Scheme keymaps, transients, and Major/Minor/Polymode registries
//! over native C-ABI FFI directly into `MyGoBridgeHost` gRPC snapshots.
//!
//! Startup architecture (zero-wait sidecar boot):
//! 1. Starts the multi-threaded Tokio HTTP/2 gRPC server immediately and prints
//!    `READY <addr>` in < 25ms so Desktop/Mobile clients connect without blocking.
//! 2. Initializes the embedded Gambit/Gerbil Scheme VM strictly on the main OS
//!    thread, projects authoritative Scheme keymaps, transients, commands, and
//!    mode records over native C-ABI FFI, and hot-publishes them live onto the
//!    open `WatchWorkbench` gRPC stream.

#[cfg(not(feature = "external-program"))]
fn resolve_scheme_dir() -> std::path::PathBuf {
    if let Ok(explicit) = std::env::var("MARLIN_SCHEME_DIR") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            return std::path::PathBuf::from(trimmed);
        }
    }
    if let Ok(ws_root) = std::env::var("MARLIN_BRIDGE_WORKSPACE_ROOT") {
        let trimmed = ws_root.trim();
        if !trimmed.is_empty() {
            let cand = std::path::PathBuf::from(trimmed).join("scheme");
            if cand.join("gerbil.pkg").is_file() {
                return cand;
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let cand = cwd.join("scheme");
        if cand.join("gerbil.pkg").is_file() {
            return cand;
        }
    }
    marlin_gerbil_scheme::gerbil_package_root()
}

#[cfg(not(feature = "external-program"))]
fn configure_gerbil_loadpath_before_threads(scheme_dir: &std::path::Path) {
    let lib_dir = scheme_dir.join(".gerbil").join("lib");
    let loadpath_missing = std::env::var("GERBIL_LOADPATH")
        .map(|v| v.trim().is_empty())
        .unwrap_or(true);
    if loadpath_missing && lib_dir.is_dir() {
        // SAFETY: Called strictly on the main thread before spawning any background threads.
        unsafe {
            std::env::set_var("GERBIL_LOADPATH", &lib_dir);
        }
    }
}

#[cfg(not(feature = "external-program"))]
fn project_live_scheme_surface(
    scheme_dir: &std::path::Path,
) -> Option<marlin_agent_app_server::mygo_host::SchemeUiSurfaceBundle> {
    use gerbil_scheme::{GerbilRuntime, PackageResolutionProgress};
    use std::time::Instant;

    if !scheme_dir.join("gerbil.pkg").is_file() {
        eprintln!(
            "marlin-app-server: scheme package root not found at {}; running without Scheme UI projection",
            scheme_dir.display()
        );
        return None;
    }

    let scheme_dir_str = match scheme_dir.to_str() {
        Some(s) => s,
        None => {
            eprintln!("marlin-app-server: scheme dir path is not valid UTF-8");
            return None;
        }
    };

    let t0 = Instant::now();
    let runtime = match GerbilRuntime::initialize() {
        Ok(rt) => rt,
        Err(err) => {
            eprintln!("marlin-app-server: failed to initialize embedded Gerbil VM: {err}");
            return None;
        }
    };
    let _ = runtime.set_package_resolution_progress(PackageResolutionProgress::Quiet);

    match marlin_gerbil_scheme::scheme_ui_keymap::project_scheme_ui_surface(&runtime, scheme_dir_str)
    {
        Ok(surface) => {
            eprintln!(
                "marlin-app-server: projected authoritative Scheme UI surface via C-ABI FFI in {:.1}ms (commands={}, keymap={}, transients={}, mode_records={})",
                t0.elapsed().as_secs_f64() * 1000.0,
                surface.commands.len(),
                surface.keymap.len(),
                surface.transients.len(),
                surface.mode_records.len(),
            );
            Some(marlin_agent_app_server::mygo_host::SchemeUiSurfaceBundle {
                commands: surface.commands,
                keymap: surface.keymap,
                transients: surface.transients,
                mode_records: surface.mode_records,
            })
        }
        Err(err) => {
            eprintln!(
                "marlin-app-server: Scheme UI keymap/mode FFI projection failed (build `just scheme-build`?): {err}"
            );
            None
        }
    }
}

#[cfg(not(feature = "external-program"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scheme_dir = resolve_scheme_dir();
    configure_gerbil_loadpath_before_threads(&scheme_dir);

    // 1. Launch the multi-threaded Tokio HTTP/2 gRPC server immediately so `READY <addr>`
    //    is emitted in milliseconds without blocking sidecar handshake on Gerbil VM boot.
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let server_thread = std::thread::Builder::new()
        .name("marlin-grpc-server".to_owned())
        .spawn(move || -> Result<(), String> {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("tokio runtime: {e}"))?;
            let res = rt
                .block_on(marlin_agent_app_server::mygo_host::run_cli_with_host_ready(
                    None,
                    Some(ready_tx),
                ))
                .map_err(|e| e.to_string());
            if res.is_ok() {
                // Exit cleanly once graceful gRPC & Turso flush completes, even if
                // main-thread Scheme VM warm-up is still in flight.
                std::process::exit(0);
            }
            res
        })?;

    // 2. Once the HTTP/2 listener is bound and live, project authoritative Scheme keymaps,
    //    transients, commands, and Major/Minor/Polymode records strictly on the main OS
    //    thread and hot-broadcast them over the live WatchWorkbench gRPC stream.
    if let Ok(host) = ready_rx.recv()
        && let Some(surface) = project_live_scheme_surface(&scheme_dir)
    {
        host.set_scheme_mode_records(surface.mode_records);
        host.set_scheme_ui_surface(surface.commands, surface.keymap, surface.transients);
    }

    match server_thread.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(err)) => Err(err.into()),
        Err(_) => Err("marlin-grpc-server thread panicked".into()),
    }
}

#[cfg(feature = "external-program")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    Err("marlin-app-server requires dynamic Gerbil VM mode (without `external-program`)".into())
}
