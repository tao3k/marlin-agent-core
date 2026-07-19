use super::support::{
    write_agent_policy_routing_native_aot_scms, write_deck_runtime_native_aot_scms,
    write_empty_file, write_executable,
};
use marlin_agent_protocol::GraphNativeAbiReadinessStatus;
use marlin_gerbil_scheme::{
    GerbilDeckRuntimeNativeAotBuildStatus, GerbilDeckRuntimeNativeAotConfig,
    GerbilDeckRuntimeNativeCargoDirectiveKind, GerbilDeckRuntimeNativeStaticLinkStatus,
    GerbilDeckRuntimeNativeSymbolAuditMethod,
};
use std::fs;
use tempfile::Builder;

static GERBIL_TOOLCHAIN_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct EnvVarGuard {
    name: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    fn set(name: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = std::env::var_os(name);
        // SAFETY: Gerbil toolchain environment mutations in this test file are
        // serialized by GERBIL_TOOLCHAIN_ENV_LOCK and restored by Drop.
        unsafe {
            std::env::set_var(name, value);
        }
        Self { name, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        // SAFETY: the guard is only constructed while the test holds
        // GERBIL_TOOLCHAIN_ENV_LOCK, so restoration is serialized.
        unsafe {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.name, previous);
            } else {
                std::env::remove_var(self.name);
            }
        }
    }
}

#[test]
#[cfg(unix)]
fn deck_runtime_native_aot_build_runs_link_unit_runner() {
    let _env_lock = GERBIL_TOOLCHAIN_ENV_LOCK.lock().expect("lock Gerbil env");
    let root = Builder::new()
        .prefix("marlin-gerbil-native-aot-build-")
        .tempdir()
        .expect("create root");
    let gerbil_prefix = root.path().join("gerbil-prefix");
    fs::create_dir_all(gerbil_prefix.join("bin")).expect("create Gerbil bin");
    fs::create_dir_all(gerbil_prefix.join("lib/gerbil")).expect("create Gerbil lib");
    write_empty_file(&gerbil_prefix.join("include/gambit.h"));
    write_empty_file(&gerbil_prefix.join("lib/libgambit.a"));

    let gsc = gerbil_prefix.join("bin/gsc");
    let _gsc_env = EnvVarGuard::set("GERBIL_GSC", &gsc);
    let nm = root.path().join("toolchain/nm");
    let _compiled_runtime_scm = write_deck_runtime_native_aot_scms(root.path());
    let expected_prefix = gerbil_prefix.to_string_lossy();
    write_executable(
        &gsc,
        format!(
            r#"#!/bin/sh
set -eu
if [ "${{GERBIL_HOME:-}}" != "{expected_prefix}" ]; then
  echo "expected GERBIL_HOME={expected_prefix}, got ${{GERBIL_HOME:-}}" >&2
  exit 71
fi
case "${{GAMBOPT:-}}" in
  *"~~bin={expected_prefix}/bin"* ) ;;
  * ) echo "missing ~~bin in GAMBOPT=${{GAMBOPT:-}}" >&2; exit 72 ;;
esac
case "${{GAMBOPT:-}}" in
  *"~~lib={expected_prefix}/lib"* ) ;;
  * ) echo "missing ~~lib in GAMBOPT=${{GAMBOPT:-}}" >&2; exit 73 ;;
esac
case "${{GAMBOPT:-}}" in
  *"~~include={expected_prefix}/include"* ) ;;
  * ) echo "missing ~~include in GAMBOPT=${{GAMBOPT:-}}" >&2; exit 74 ;;
esac
mode=""
source=""
skip_next=0
for arg in "$@"; do
  if [ "$skip_next" = "1" ]; then
    skip_next=0
    continue
  fi
  if [ "$arg" = "-cc" ] || [ "$arg" = "-cc-options" ] || [ "$arg" = "-target" ]; then
    skip_next=1
    continue
  fi
  if [ "$arg" = "-obj" ] || [ "$arg" = "-link" ]; then
    mode="$arg"
  else
    source="$arg"
  fi
done
dir=$(dirname "$source")
base=$(basename "$source")
stem=${{base%.*}}
if [ "$mode" = "-link" ]; then
  : > "$dir/deck-runtime-native~0_.c"
else
  : > "$dir/$stem.o"
fi
"#,
            expected_prefix = expected_prefix
        )
        .as_str(),
    );
    write_executable(
        &nm,
        r#"#!/bin/sh
set -eu
printf '00000000 T marlin_deck_runtime_initialize\n'
printf '00000000 T marlin_deck_runtime_select_model_route\n'
"#,
    );

    let receipt = GerbilDeckRuntimeNativeAotConfig::new(root.path())
        .with_gsc("gsc")
        .with_c_compiler("clang")
        .with_symbol_auditor(nm)
        .with_gambit_link_search_dir(root.path().join("lib"))
        .build_link_unit();

    assert_eq!(
        receipt.status,
        GerbilDeckRuntimeNativeAotBuildStatus::LinkUnitReady
    );
    assert!(receipt.plan.compiled_runtime_scm.is_file());
    assert!(receipt.plan.object.is_file());
    assert!(receipt.plan.link_c_source.is_file());
    assert!(receipt.plan.link_object.is_file());
    assert_eq!(
        receipt
            .gsc_compile_object
            .as_ref()
            .and_then(|command| command.status_code),
        Some(0)
    );
    assert_eq!(
        receipt
            .symbol_audit
            .as_ref()
            .and_then(|command| command.status_code),
        Some(0)
    );
    assert_eq!(
        receipt.symbol_audit_method,
        Some(GerbilDeckRuntimeNativeSymbolAuditMethod::SymbolTableCommand)
    );
    assert_eq!(
        receipt
            .gsc_generate_link_source
            .as_ref()
            .and_then(|command| command.status_code),
        Some(0)
    );
    assert_eq!(
        receipt
            .gsc_compile_link_object
            .as_ref()
            .and_then(|command| command.status_code),
        Some(0)
    );
    assert!(receipt.missing_symbols.is_empty());

    let link_plan = receipt.static_link_plan();
    assert_eq!(
        link_plan.status,
        GerbilDeckRuntimeNativeStaticLinkStatus::Ready
    );
    assert_eq!(link_plan.module_object, receipt.plan.object);
    assert_eq!(link_plan.link_object, receipt.plan.link_object);
    assert_eq!(
        link_plan
            .link_libraries
            .iter()
            .map(|library| library.as_str())
            .collect::<Vec<_>>(),
        ["gambit"]
    );
    assert_eq!(link_plan.link_search_dirs, [root.path().join("lib")]);
    assert_eq!(
        link_plan
            .cargo_directives
            .iter()
            .map(|directive| directive.kind)
            .collect::<Vec<_>>(),
        [
            GerbilDeckRuntimeNativeCargoDirectiveKind::RustcLinkArg,
            GerbilDeckRuntimeNativeCargoDirectiveKind::RustcLinkArg,
            GerbilDeckRuntimeNativeCargoDirectiveKind::RustcLinkArg,
            GerbilDeckRuntimeNativeCargoDirectiveKind::RustcLinkSearch,
            GerbilDeckRuntimeNativeCargoDirectiveKind::RustcLinkLib
        ]
    );
    assert!(
        link_plan
            .cargo_directives
            .iter()
            .any(|directive| directive.line().contains("cargo:rustc-link-lib=gambit"))
    );
}

#[test]
#[cfg(unix)]
fn agent_policy_routing_native_aot_build_projects_graph_readiness_receipt() {
    let root = Builder::new()
        .prefix("marlin-gerbil-agent-policy-routing-native-aot-build-")
        .tempdir()
        .expect("create root");
    let gsc = root.path().join("toolchain/gsc");
    let nm = root.path().join("toolchain/nm");
    let _compiled_runtime_scm = write_agent_policy_routing_native_aot_scms(root.path());
    write_executable(
        &gsc,
        r#"#!/bin/sh
set -eu
mode=""
source=""
skip_next=0
for arg in "$@"; do
  if [ "$skip_next" = "1" ]; then
    skip_next=0
    continue
  fi
  if [ "$arg" = "-cc" ] || [ "$arg" = "-cc-options" ] || [ "$arg" = "-target" ]; then
    skip_next=1
    continue
  fi
  if [ "$arg" = "-obj" ] || [ "$arg" = "-link" ]; then
    mode="$arg"
  else
    source="$arg"
  fi
done
dir=$(dirname "$source")
base=$(basename "$source")
stem=${base%.*}
if [ "$mode" = "-link" ]; then
  : > "$dir/agent-policy-routing-native~0_.c"
else
  : > "$dir/$stem.o"
fi
"#,
    );
    write_executable(
        &nm,
        r#"#!/bin/sh
set -eu
printf '00000000 T marlin_agent_policy_routing_initialize\n'
printf '00000000 T marlin_agent_policy_routing_select_edges\n'
"#,
    );

    let receipt = GerbilDeckRuntimeNativeAotConfig::agent_policy_routing(root.path())
        .with_gsc(gsc)
        .with_symbol_auditor(nm)
        .build_link_unit();

    assert_eq!(
        receipt.status,
        GerbilDeckRuntimeNativeAotBuildStatus::LinkUnitReady
    );
    assert!(receipt.plan.object.is_file());
    assert!(receipt.plan.link_c_source.is_file());
    assert!(receipt.plan.link_object.is_file());
    assert!(receipt.missing_symbols.is_empty());

    let readiness = receipt.graph_native_abi_readiness_receipt();
    assert_eq!(readiness.status, GraphNativeAbiReadinessStatus::Ready);
    assert_eq!(readiness.required_symbol_count, 2);
    assert_eq!(readiness.available_symbol_count, 2);
    assert_eq!(readiness.matched_symbol_count, 2);
    assert!(readiness.missing_symbols.is_empty());
}

#[test]
#[cfg(unix)]
fn deck_runtime_native_aot_build_rejects_missing_link_source() {
    let root = Builder::new()
        .prefix("marlin-gerbil-native-aot-missing-link-source-")
        .tempdir()
        .expect("create root");
    let gsc = root.path().join("toolchain/gsc");
    let _compiled_runtime_scm = write_deck_runtime_native_aot_scms(root.path());
    write_executable(
        &gsc,
        r#"#!/bin/sh
set -eu
mode=""
source=""
skip_next=0
for arg in "$@"; do
  if [ "$skip_next" = "1" ]; then
    skip_next=0
    continue
  fi
  if [ "$arg" = "-cc" ] || [ "$arg" = "-cc-options" ] || [ "$arg" = "-target" ]; then
    skip_next=1
    continue
  fi
  if [ "$arg" = "-obj" ] || [ "$arg" = "-link" ]; then
    mode="$arg"
  else
    source="$arg"
  fi
done
dir=$(dirname "$source")
base=$(basename "$source")
stem=${base%.*}
if [ "$mode" = "-obj" ]; then
  : > "$dir/$stem.o"
fi
"#,
    );

    let receipt = GerbilDeckRuntimeNativeAotConfig::new(root.path())
        .with_gsc(gsc)
        .build_link_unit();

    assert_eq!(
        receipt.status,
        GerbilDeckRuntimeNativeAotBuildStatus::LinkSourceMissing
    );
    assert_eq!(
        receipt
            .gsc_generate_link_source
            .as_ref()
            .and_then(|command| command.status_code),
        Some(0)
    );
    assert!(receipt.gsc_compile_link_object.is_none());
    assert!(receipt.symbol_audit.is_none());
}

#[test]
#[cfg(unix)]
fn deck_runtime_native_aot_build_rejects_missing_link_object() {
    let root = Builder::new()
        .prefix("marlin-gerbil-native-aot-missing-link-object-")
        .tempdir()
        .expect("create root");
    let gsc = root.path().join("toolchain/gsc");
    let _compiled_runtime_scm = write_deck_runtime_native_aot_scms(root.path());
    write_executable(
        &gsc,
        r#"#!/bin/sh
set -eu
mode=""
source=""
skip_next=0
for arg in "$@"; do
  if [ "$skip_next" = "1" ]; then
    skip_next=0
    continue
  fi
  if [ "$arg" = "-cc" ] || [ "$arg" = "-cc-options" ] || [ "$arg" = "-target" ]; then
    skip_next=1
    continue
  fi
  if [ "$arg" = "-obj" ] || [ "$arg" = "-link" ]; then
    mode="$arg"
  else
    source="$arg"
  fi
done
dir=$(dirname "$source")
base=$(basename "$source")
stem=${base%.*}
if [ "$mode" = "-link" ]; then
  : > "$dir/deck-runtime-native~0_.c"
elif [ "${base##*.}" != "c" ]; then
  : > "$dir/$stem.o"
fi
"#,
    );

    let receipt = GerbilDeckRuntimeNativeAotConfig::new(root.path())
        .with_gsc(gsc)
        .build_link_unit();

    assert_eq!(
        receipt.status,
        GerbilDeckRuntimeNativeAotBuildStatus::LinkObjectMissing
    );
    assert_eq!(
        receipt
            .gsc_compile_link_object
            .as_ref()
            .and_then(|command| command.status_code),
        Some(0)
    );
    assert!(receipt.symbol_audit.is_none());
}

#[test]
#[cfg(unix)]
fn deck_runtime_native_aot_build_rejects_object_missing_required_symbols() {
    let root = Builder::new()
        .prefix("marlin-gerbil-native-aot-missing-symbols-")
        .tempdir()
        .expect("create root");
    let gsc = root.path().join("toolchain/gsc");
    let nm = root.path().join("toolchain/nm");
    let _compiled_runtime_scm = write_deck_runtime_native_aot_scms(root.path());
    write_executable(
        &gsc,
        r#"#!/bin/sh
set -eu
mode=""
source=""
skip_next=0
for arg in "$@"; do
  if [ "$skip_next" = "1" ]; then
    skip_next=0
    continue
  fi
  if [ "$arg" = "-cc" ] || [ "$arg" = "-cc-options" ] || [ "$arg" = "-target" ]; then
    skip_next=1
    continue
  fi
  if [ "$arg" = "-obj" ] || [ "$arg" = "-link" ]; then
    mode="$arg"
  else
    source="$arg"
  fi
done
dir=$(dirname "$source")
base=$(basename "$source")
stem=${base%.*}
if [ "$mode" = "-link" ]; then
  : > "$dir/deck-runtime-native~0_.c"
else
  : > "$dir/$stem.o"
fi
"#,
    );
    write_executable(
        &nm,
        r#"#!/bin/sh
set -eu
printf '00000000 T unrelated_symbol\n'
"#,
    );

    let receipt = GerbilDeckRuntimeNativeAotConfig::new(root.path())
        .with_gsc(gsc)
        .with_symbol_auditor(nm)
        .build_link_unit();

    assert_eq!(
        receipt.status,
        GerbilDeckRuntimeNativeAotBuildStatus::RequiredSymbolsMissing
    );
    assert_eq!(
        receipt.symbol_audit_method,
        Some(GerbilDeckRuntimeNativeSymbolAuditMethod::SymbolTableCommand)
    );
    assert_eq!(
        receipt
            .missing_symbols
            .iter()
            .map(|symbol| symbol.as_str())
            .collect::<Vec<_>>(),
        [
            "marlin_deck_runtime_initialize",
            "marlin_deck_runtime_select_model_route"
        ]
    );
}
