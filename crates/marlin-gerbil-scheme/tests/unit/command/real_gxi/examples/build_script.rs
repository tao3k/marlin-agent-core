use super::support::test_root;
use super::support::{MARLIN_REQUIRE_REAL_GXI_ENV, local_gxi};
use marlin_gerbil_scheme::{
    GERBIL_LOADPATH_ENV, GerbilDeckRuntimeNativeAotBuildStatus, GerbilDeckRuntimeNativeAotConfig,
    GerbilDeckRuntimeNativeStaticLinkStatus, default_gerbil_gxc_program,
    default_gerbil_gxpkg_program, gerbil_runtime_loadpath_with_dependencies,
    resolve_gerbil_executable, write_gerbil_runtime_assets,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_build_script_compiles_runtime_assets() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = test_root("runtime-build-script");
    write_gerbil_runtime_assets(root.path()).expect("write gerbil runtime assets");
    run_gerbil_build_script_compile(&gxi, root.path());
}

#[test]
#[ignore = "requires a local Gerbil gxi executable and installed package dependencies"]
fn command_compiler_real_gxi_build_script_runs_scheme_to_rust_bridge_smoke() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = test_root("runtime-scheme-to-rust-bridge");
    write_gerbil_runtime_assets(root.path()).expect("write gerbil runtime assets");
    run_gerbil_build_script_compile(&gxi, root.path());

    let stdout = run_scheme_to_rust_bridge_smoke(&gxi, root.path(), 8);
    assert_scheme_to_rust_bridge_smoke(&stdout, 8);
}

#[test]
#[ignore = "requires a local Gerbil gxi executable and installed package dependencies"]
fn command_compiler_real_gxi_build_script_runs_config_interface_profile_projection_smoke() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = test_root("runtime-config-interface-profile-projection");
    write_gerbil_runtime_assets(root.path()).expect("write gerbil runtime assets");
    run_gerbil_build_script_compile(&gxi, root.path());

    let stdout = run_config_interface_profile_projection_smoke(&gxi, root.path());
    assert_config_interface_profile_projection_smoke(&stdout);
}

#[test]
#[ignore = "requires a local Gerbil gxi/gsc toolchain and C compiler"]
fn command_compiler_real_gxi_deck_runtime_native_links_and_runs_abi_smoke() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let Some(gxc) = local_gxc() else {
        return;
    };
    let Some(gxpkg) = local_gxpkg() else {
        return;
    };
    let root = test_root("runtime-native-linked-abi-smoke");
    write_gerbil_runtime_assets(root.path()).expect("write gerbil runtime assets");

    let compiled_runtime_scm = run_gerbil_compile_native_aot(&gxpkg, &gxc, root.path());
    let gerbil_home = gerbil_home_from_gxi(&gxi);
    let config = GerbilDeckRuntimeNativeAotConfig::new(root.path())
        .with_compiled_runtime_scm(compiled_runtime_scm)
        .with_gambit_link_search_dir(gerbil_home.join("lib"));
    let config = if cfg!(target_os = "macos") {
        config.with_c_compiler("clang")
    } else {
        config
    };
    let receipt = config.build_link_unit();
    if receipt.status != GerbilDeckRuntimeNativeAotBuildStatus::LinkUnitReady {
        let message = format!(
            "native AOT link unit build failed: {receipt:?}\nmodule nm:\n{}\nlink nm:\n{}",
            nm_output(&receipt.plan.object),
            nm_output(&receipt.plan.link_object)
        );
        if std::env::var_os(MARLIN_REQUIRE_REAL_GXI_ENV).is_some() {
            panic!("{message}");
        }
        eprintln!("{message}");
        return;
    }

    assert!(
        receipt.plan.compiled_runtime_scm.is_file(),
        "missing compiled runtime Scheme at {}",
        receipt.plan.compiled_runtime_scm.display()
    );
    assert!(
        receipt.plan.object.is_file(),
        "missing native module object at {}",
        receipt.plan.object.display()
    );
    assert!(
        receipt.plan.link_object.is_file(),
        "missing native link object at {}",
        receipt.plan.link_object.display()
    );
    assert_eq!(
        receipt.static_link_plan().status,
        GerbilDeckRuntimeNativeStaticLinkStatus::Ready
    );
    run_linked_deck_runtime_abi_smoke(root.path(), &receipt, &gerbil_home);
}

fn nm_output(path: &Path) -> String {
    if !path.is_file() {
        return format!("{} is missing", path.display());
    }
    let output = Command::new("nm")
        .arg(path)
        .output()
        .expect("run nm for native AOT diagnostic");
    format!(
        "status={:?}\nstdout={}\nstderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn run_gerbil_build_script_compile(gxi: &Path, root: &Path) {
    let output = Command::new(gxi)
        .env(
            GERBIL_LOADPATH_ENV,
            gerbil_runtime_loadpath_with_dependencies(root),
        )
        .current_dir(root)
        .arg(root.join("build.ss"))
        .arg("compile")
        .output()
        .expect("run real gxi build script for native ABI smoke");

    assert!(
        output.status.success(),
        "gxi build script failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_scheme_to_rust_bridge_smoke(gxi: &Path, root: &Path, iterations: u64) -> String {
    let expression = format!(
        r#"(begin
  (import :clan/poo/object
          (only-in :marlin/deck-runtime-script-performance
                   deck-runtime-script-performance-context
                   deck-runtime-script-performance-count-runs
                   deck-runtime-script-performance-run-batch))
  (def iterations {iterations})
  (def context (deck-runtime-script-performance-context))
  (def metrics (deck-runtime-script-performance-run-batch iterations context))
  (display "schema=marlin-gerbil.scheme-to-rust-bridge-smoke.v1") (newline)
  (display "kind=") (display (.get metrics kind)) (newline)
  (display "script-id=") (display (.get metrics script-id)) (newline)
  (display "iterations=") (display (.get metrics iterations)) (newline)
  (display "runs=") (display (.get metrics runs)) (newline)
  (display "count=") (display (deck-runtime-script-performance-count-runs iterations context)) (newline)
  (display "elapsed-us=") (display (.get metrics elapsed-us)) (newline))"#
    );
    let output = Command::new(gxi)
        .env(
            GERBIL_LOADPATH_ENV,
            gerbil_runtime_loadpath_with_dependencies(root),
        )
        .current_dir(root)
        .arg("-e")
        .arg(expression)
        .output()
        .expect("run real gxi Scheme-to-Rust bridge smoke");

    assert!(
        output.status.success(),
        "gxi Scheme-to-Rust bridge smoke failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("Scheme-to-Rust bridge stdout is UTF-8")
}

fn assert_scheme_to_rust_bridge_smoke(stdout: &str, iterations: u64) {
    let expected_lines = [
        "schema=marlin-gerbil.scheme-to-rust-bridge-smoke.v1".to_string(),
        "kind=marlin-deck-runtime.script-batch-metrics.v1".to_string(),
        "script-id=performance-script".to_string(),
        format!("iterations={iterations}"),
        format!("runs={iterations}"),
        format!("count={iterations}"),
    ];
    let lines = stdout.lines().collect::<Vec<_>>();
    for expected in expected_lines {
        assert!(
            lines.contains(&expected.as_str()),
            "missing Scheme-to-Rust bridge receipt line `{expected}` in stdout:\n{stdout}"
        );
    }
    let elapsed_us = lines
        .iter()
        .find_map(|line| line.strip_prefix("elapsed-us="))
        .expect("Scheme-to-Rust bridge receipt should include elapsed-us")
        .parse::<u64>()
        .expect("elapsed-us should be an unsigned integer");
    assert!(
        elapsed_us < 1_000_000,
        "Scheme-to-Rust bridge smoke should stay below 1s for {iterations} iterations, got {elapsed_us}us"
    );
    assert!(
        !stdout.contains("{\""),
        "Scheme-to-Rust bridge smoke must not use JSON output:\n{stdout}"
    );
}

fn run_config_interface_profile_projection_smoke(gxi: &Path, root: &Path) -> String {
    let expression = r#"(begin
  (import :clan/poo/object
          :config-interface/lib)
  (def modules (marlinLoopPolicyProjectionModules))
  (def receipts (marlinLoopPolicyProfileCompilerReceipts))
  (def (emit key value)
    (display key) (display "=") (display value) (newline))
  (emit "schema" "marlin-gerbil.config-interface.profile-compiler-receipts.v1")
  (emit "module-count" (vector-length modules))
  (emit "count" (vector-length receipts))
  (let loop-modules ((index 0))
    (when (< index (vector-length modules))
      (let* ((module (vector-ref modules index))
             (compiler-receipt (.get module compiler-receipt))
             (loop-program (.get compiler-receipt loop-program))
             (capability-lanes (.get module poo-flow-capability-lanes))
             (prefix (string-append "module." (number->string index) ".")))
        (emit (string-append prefix "module-id") (.get module module-id))
        (emit (string-append prefix "source-module") (.get module source-module))
        (emit (string-append prefix "poo-flow-module") (.get module poo-flow-module))
        (emit (string-append prefix "capability-lane-count")
              (vector-length capability-lanes))
        (emit (string-append prefix "primary-capability-lane")
              (vector-ref capability-lanes 0))
        (emit (string-append prefix "rust-type") (.get module rust-type))
        (emit (string-append prefix "profile-id") (.get module profile-id))
        (emit (string-append prefix "compiler-receipt-profile-id")
              (.get compiler-receipt profile-id))
        (emit (string-append prefix "program-id") (.get loop-program program_id))
        (loop-modules (+ index 1)))))
  (let loop ((index 0))
    (when (< index (vector-length receipts))
      (let* ((receipt (vector-ref receipts index))
             (resolved-policy-pack (.get receipt resolved-policy-pack))
             (loop-program (.get receipt loop-program))
             (prefix (string-append "receipt." (number->string index) ".")))
        (emit (string-append prefix "profile-id") (.get receipt profile-id))
        (emit (string-append prefix "compiler-owner") (.get receipt compiler-owner))
        (emit (string-append prefix "scheme-boundary") (.get receipt scheme-boundary))
        (emit (string-append prefix "serialization-boundary") (.get receipt serialization-boundary))
        (emit (string-append prefix "policy-epoch") (.get resolved-policy-pack policy_epoch))
        (emit (string-append prefix "program-id") (.get loop-program program_id))
        (emit (string-append prefix "transition-count")
              (vector-length (.get loop-program transitions)))
        (loop (+ index 1))))))"#;
    let output = Command::new(gxi)
        .env(
            GERBIL_LOADPATH_ENV,
            gerbil_runtime_loadpath_with_dependencies(root),
        )
        .current_dir(root)
        .arg("-e")
        .arg(expression)
        .output()
        .expect("run real gxi config-interface profile projection smoke");

    assert!(
        output.status.success(),
        "gxi config-interface profile projection smoke failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("config-interface profile projection stdout is UTF-8")
}

fn assert_config_interface_profile_projection_smoke(stdout: &str) {
    let expected_lines = [
        "schema=marlin-gerbil.config-interface.profile-compiler-receipts.v1",
        "module-count=9",
        "count=9",
        "module.0.module-id=poo-flow.loop-engine.real-repair-001",
        "module.0.profile-id=real-repair-001/reactive-tool-loop",
        "module.0.program-id=real-repair-001-scripted-loop",
        "module.1.module-id=poo-flow.loop-engine.failure-retry",
        "module.1.profile-id=marlin-failure-retry-profile/typed-recovery",
        "module.1.program-id=failure-retry-typed-recovery",
        "module.2.module-id=poo-flow.loop-engine.policy-combination-matrix",
        "module.2.profile-id=policy-combination/memory-rewrite-checker",
        "module.2.program-id=policy-combination-memory-rewrite-checker",
        "module.3.module-id=poo-flow.loop-engine.real-policy-001-sandbox-denylist",
        "module.3.profile-id=real-policy-001/sandbox-denylist",
        "module.4.module-id=poo-flow.loop-engine.real-policy-001-tool-sandbox",
        "module.4.profile-id=real-policy-001/tool-sandbox",
        "module.4.program-id=real-tool-sandbox-loop",
        "module.5.module-id=poo-flow.loop-engine.real-policy-002-retry-budget",
        "module.5.profile-id=real-policy-002/retry-budget",
        "module.5.program-id=real-policy-002-retry-budget",
        "module.6.module-id=poo-flow.loop-engine.real-policy-003-maker-checker",
        "module.6.profile-id=real-policy-003/maker-checker",
        "module.6.program-id=real-policy-003-maker-checker",
        "module.7.module-id=poo-flow.loop-engine.real-policy-004-dynamic-rewrite",
        "module.7.profile-id=real-policy-004/dynamic-rewrite",
        "module.7.program-id=real-policy-004-dynamic-rewrite",
        "module.8.module-id=poo-flow.loop-engine.real-policy-005-memory-recall",
        "module.8.profile-id=real-policy-005/memory-recall",
        "module.8.program-id=real-policy-005-memory-recall",
        "receipt.0.profile-id=real-repair-001/reactive-tool-loop",
        "receipt.0.program-id=real-repair-001-scripted-loop",
        "receipt.1.profile-id=marlin-failure-retry-profile/typed-recovery",
        "receipt.1.program-id=failure-retry-typed-recovery",
        "receipt.2.profile-id=policy-combination/memory-rewrite-checker",
        "receipt.2.program-id=policy-combination-memory-rewrite-checker",
        "receipt.3.profile-id=real-policy-001/sandbox-denylist",
        "receipt.4.profile-id=real-policy-001/tool-sandbox",
        "receipt.4.program-id=real-tool-sandbox-loop",
        "receipt.5.profile-id=real-policy-002/retry-budget",
        "receipt.5.program-id=real-policy-002-retry-budget",
        "receipt.6.profile-id=real-policy-003/maker-checker",
        "receipt.6.program-id=real-policy-003-maker-checker",
        "receipt.7.profile-id=real-policy-004/dynamic-rewrite",
        "receipt.7.program-id=real-policy-004-dynamic-rewrite",
        "receipt.8.profile-id=real-policy-005/memory-recall",
        "receipt.8.program-id=real-policy-005-memory-recall",
    ];
    let lines = stdout.lines().collect::<Vec<_>>();
    for expected in expected_lines {
        assert!(
            lines.contains(&expected),
            "missing config-interface profile projection receipt line `{expected}` in stdout:\n{stdout}"
        );
    }
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("module.") && line.contains(".module-id="))
            .count(),
        9
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("module.") && line.contains(".profile-id="))
            .count(),
        9
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("module.")
                && line.contains(".compiler-receipt-profile-id="))
            .count(),
        9
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("module.") && line.contains(".program-id="))
            .count(),
        9
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("module.")
                && line.contains(
                    ".rust-type=marlin.config-interface.poo.loop-program-compiler-receipt.v1"
                ))
            .count(),
        9
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("receipt.") && line.contains(".profile-id="))
            .count(),
        9
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("receipt.")
                && line.contains(".compiler-owner=gerbil-poo-flow"))
            .count(),
        9
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("receipt.")
                && line.contains(".scheme-boundary=scheme-types-to-rust-types"))
            .count(),
        9
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("receipt.")
                && line.contains(".serialization-boundary=rust-owned-cli-trace-cross-process"))
            .count(),
        9
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("receipt.") && line.contains(".program-id="))
            .count(),
        9
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("receipt.") && line.contains(".transition-count="))
            .count(),
        9
    );
    assert!(
        !stdout.contains("{\""),
        "config-interface profile projection smoke must not use JSON output:\n{stdout}"
    );
}

fn run_gerbil_compile_native_aot(gxpkg: &Path, gxc: &Path, root: &Path) -> PathBuf {
    compile_gerbil_native_aot_source(
        gxpkg,
        gxc,
        root,
        "src/marlin/_deck-runtime-native.ssi",
        "native AOT runtime binding artifact",
    );
    let output = Command::new(gxpkg)
        .current_dir(root)
        .arg("env")
        .arg(gxc)
        .arg("-target")
        .arg("C")
        .arg("-s")
        .arg("-S")
        .arg("-O")
        .arg("src/marlin/deck-runtime-native.ss")
        .output()
        .expect("compile native AOT runtime artifact");

    assert!(
        output.status.success(),
        "native AOT compile failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let gerbil_pkg = root.join("gerbil.pkg");
    stage_compiled_scheme(
        root,
        &gerbil_pkg,
        "_deck-runtime-native~0.scm",
        ".gerbil/native/_deck-runtime-native~0.scm",
    );
    stage_compiled_scheme(
        root,
        &gerbil_pkg,
        "deck-runtime-native~0.scm",
        ".gerbil/native/deck-runtime-native~0.scm",
    )
}

fn compile_gerbil_native_aot_source(
    gxpkg: &Path,
    gxc: &Path,
    root: &Path,
    source: &str,
    label: &str,
) {
    let output = Command::new(gxpkg)
        .current_dir(root)
        .arg("env")
        .arg(gxc)
        .arg("-target")
        .arg("C")
        .arg("-s")
        .arg("-S")
        .arg("-O")
        .arg(source)
        .output()
        .unwrap_or_else(|error| panic!("compile {label}: {error}"));

    assert!(
        output.status.success(),
        "{label} compile failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn stage_compiled_scheme(
    root: &Path,
    gerbil_pkg: &Path,
    builder_file_name: &str,
    staged_relative_path: &str,
) -> PathBuf {
    let builder_scm = compiled_scheme_candidate(gerbil_pkg, builder_file_name);
    let compiled_runtime_scm = root.join(staged_relative_path);
    fs::create_dir_all(
        compiled_runtime_scm
            .parent()
            .expect("compiled runtime artifact has parent"),
    )
    .expect("create native AOT staging dir");
    fs::copy(&builder_scm, &compiled_runtime_scm).unwrap_or_else(|error| {
        panic!(
            "copy Gerbil local builder native AOT artifact from {} to {} failed: {error}; candidates={:?}",
            builder_scm.display(),
            compiled_runtime_scm.display(),
            compiled_scheme_candidates(gerbil_pkg)
        )
    });
    assert!(
        compiled_runtime_scm.is_file(),
        "missing staged native AOT artifact at {}; candidates={:?}",
        compiled_runtime_scm.display(),
        compiled_scheme_candidates(gerbil_pkg)
    );
    compiled_runtime_scm
}

fn compiled_scheme_candidate(gerbil_pkg: &Path, file_name: &str) -> PathBuf {
    let matches = compiled_scheme_candidates(gerbil_pkg)
        .into_iter()
        .filter(|path| path.file_name().and_then(|name| name.to_str()) == Some(file_name))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [path] => path.clone(),
        [] => panic!(
            "missing Gerbil local builder artifact {file_name}; candidates={:?}",
            compiled_scheme_candidates(gerbil_pkg)
        ),
        _ => panic!("ambiguous Gerbil local builder artifact {file_name}; candidates={matches:?}"),
    }
}

fn local_gxc() -> Option<PathBuf> {
    let configured_gxc = default_gerbil_gxc_program();
    let Some(gxc) = resolve_gerbil_executable(&configured_gxc) else {
        let message = format!(
            "skipping real gxi native AOT test because {} is missing",
            configured_gxc.display()
        );
        if std::env::var_os(MARLIN_REQUIRE_REAL_GXI_ENV).is_some() {
            panic!(
                "{message}; unset {MARLIN_REQUIRE_REAL_GXI_ENV} or set MARLIN_GERBIL_GXC to an existing executable"
            );
        }
        eprintln!("{message}");
        return None;
    };
    Some(gxc)
}

fn local_gxpkg() -> Option<PathBuf> {
    let configured_gxpkg = default_gerbil_gxpkg_program();
    let Some(gxpkg) = resolve_gerbil_executable(&configured_gxpkg) else {
        let message = format!(
            "skipping real gxi native AOT test because {} is missing",
            configured_gxpkg.display()
        );
        if std::env::var_os(MARLIN_REQUIRE_REAL_GXI_ENV).is_some() {
            panic!(
                "{message}; unset {MARLIN_REQUIRE_REAL_GXI_ENV} or set MARLIN_GERBIL_GXPKG to an existing executable"
            );
        }
        eprintln!("{message}");
        return None;
    };
    Some(gxpkg)
}

fn compiled_scheme_candidates(gerbil_pkg: &Path) -> Vec<PathBuf> {
    let builder_root = gerbil_pkg
        .parent()
        .expect("Gerbil package manifest has parent")
        .join(".gerbil");
    let mut stack = vec![builder_root];
    let mut candidates = Vec::new();
    while let Some(path) = stack.pop() {
        let Ok(entries) = fs::read_dir(path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if file_name.contains("deck-runtime-native") && file_name.ends_with(".scm") {
                candidates.push(path);
            }
        }
    }
    candidates.sort();
    candidates
}

fn gerbil_home_from_gxi(gxi: &Path) -> PathBuf {
    let output = Command::new(gxi)
        .arg("-e")
        .arg(r#"(begin (display (getenv "GERBIL_HOME")) (newline))"#)
        .output()
        .expect("query GERBIL_HOME through gxi");
    assert!(
        output.status.success(),
        "failed to query GERBIL_HOME\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let home = String::from_utf8(output.stdout)
        .expect("GERBIL_HOME should be UTF-8")
        .trim()
        .to_string();
    assert!(
        !home.is_empty() && home != "#f",
        "gxi did not report GERBIL_HOME"
    );
    PathBuf::from(home)
}

fn run_linked_deck_runtime_abi_smoke(
    root: &Path,
    receipt: &marlin_gerbil_scheme::GerbilDeckRuntimeNativeAotBuildReceipt,
    gerbil_home: &Path,
) {
    let link_plan = receipt.static_link_plan();
    assert_eq!(
        link_plan.status,
        GerbilDeckRuntimeNativeStaticLinkStatus::Ready
    );
    let source = root.join("deck-runtime-native-linked-abi-smoke.c");
    let binary = root.join("deck-runtime-native-linked-abi-smoke");
    fs::write(&source, linked_abi_smoke_source()).expect("write linked ABI smoke source");

    let mut command = Command::new(c_compiler());
    command
        .arg(&source)
        .arg("-I")
        .arg(
            link_plan
                .header
                .parent()
                .expect("native header has parent directory"),
        )
        .arg("-I")
        .arg(gerbil_home.join("include"));
    for object in &link_plan.module_objects {
        command.arg(object);
    }
    command.arg(&link_plan.link_object);
    for search_dir in &link_plan.link_search_dirs {
        command.arg("-L").arg(search_dir);
    }
    for library in &link_plan.link_libraries {
        command.arg(format!("-l{}", library.as_str()));
    }
    command.arg("-o").arg(&binary);

    let output = command.output().expect("compile linked ABI smoke runner");
    assert!(
        output.status.success(),
        "compile linked ABI smoke runner failed status={:?}\nlink nm:\n{}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        nm_output(&link_plan.link_object),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let output = Command::new(&binary)
        .output()
        .expect("run linked ABI smoke runner");
    assert!(
        output.status.success(),
        "run linked ABI smoke runner failed status={:?} binary={}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        binary.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn c_compiler() -> &'static str {
    if cfg!(target_os = "macos") {
        "clang"
    } else {
        "cc"
    }
}

fn linked_abi_smoke_source() -> &'static str {
    r#"
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "gambit.h"
#include "marlin_deck_runtime_native.h"

static MarlinDeckRuntimeUtf8 marlin_utf8(const char *value) {
  MarlinDeckRuntimeUtf8 result;
  result.ptr = (const uint8_t *)value;
  result.len = (uintptr_t)strlen(value);
  return result;
}

int main(void) {
  MarlinDeckRuntimeInitializeStatus init_status =
      marlin_deck_runtime_initialize();
  if (init_status != 0) {
    fprintf(stderr, "init_status=%d\n", init_status);
    return 10;
  }

  MarlinDeckRuntimeUtf8 command_prefixes[] = {
      marlin_utf8("cargo test"),
  };
  MarlinDeckRuntimeUtf8 agent_scopes[] = {
      marlin_utf8("sub-agent"),
  };
  MarlinDeckRuntimeModelRoutePolicy policies[] = {
      {
          marlin_utf8("cheap-test-runner"),
          marlin_utf8("openai"),
          marlin_utf8("gpt-5-mini"),
          {command_prefixes, 1},
          {agent_scopes, 1},
          marlin_utf8("forked"),
          marlin_utf8("workspace"),
      },
  };
  MarlinDeckRuntimeModelRouteRequest request = {
      MARLIN_DECK_RUNTIME_NATIVE_ABI_VERSION,
      marlin_utf8("cargo test -p marlin-gerbil-scheme"),
      marlin_utf8("sub-agent"),
      policies,
      1,
  };

  for (uintptr_t iteration = 0; iteration < 1000; ++iteration) {
    MarlinDeckRuntimeModelRouteSelection selection = {
        MARLIN_DECK_RUNTIME_NATIVE_ABI_VERSION,
        0,
        {0, 0, 0},
        MARLIN_DECK_RUNTIME_NATIVE_NO_POLICY_INDEX,
    };
    MarlinDeckRuntimeStatus status =
        marlin_deck_runtime_select_model_route(&request, &selection);
    if (status != MARLIN_DECK_RUNTIME_NATIVE_STATUS_OK) {
      fprintf(stderr, "select_status=%d\n", status);
      return 20;
    }
    if (!selection.matched || selection.policy_index != 0) {
      fprintf(stderr, "matched=%u policy_index=%lu\n", selection.matched,
              (unsigned long)selection.policy_index);
      return 21;
    }
  }

  printf("marlin-native-aot-linked-abi-smoke status=ok iterations=1000\n");
  ___cleanup();
  return 0;
}
"#
}
