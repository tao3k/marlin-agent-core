use marlin_gerbil_scheme::{
    GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PATH, GerbilDeckRuntimeNativeAotBuildStatus,
    GerbilDeckRuntimeNativeAotConfig, GerbilDeckRuntimeNativeAotStatus,
    GerbilDeckRuntimeNativeCargoDirectiveKind, GerbilDeckRuntimeNativeStaticLinkStatus,
    GerbilDeckRuntimeNativeSymbolAuditMethod, write_gerbil_runtime_assets,
};
use std::{fs, path::Path};
use tempfile::Builder;

#[test]
fn deck_runtime_native_aot_plan_records_link_unit_compile() {
    let root = Builder::new()
        .prefix("marlin-gerbil-native-aot-plan-")
        .tempdir()
        .expect("create root");
    write_gerbil_runtime_assets(root.path()).expect("write gerbil runtime assets");
    let gxc = root.path().join("toolchain/gxc");
    let gsc = root.path().join("toolchain/gsc");
    let header = root.path().join("include/marlin_deck_runtime_native.h");
    write_empty_file(&gxc);
    write_empty_file(&gsc);
    write_empty_file(&header);

    let plan = GerbilDeckRuntimeNativeAotConfig::new(root.path())
        .with_gxc(&gxc)
        .with_gsc(&gsc)
        .with_header(&header)
        .with_c_compiler("clang")
        .plan();

    assert_eq!(
        plan.status,
        GerbilDeckRuntimeNativeAotStatus::ReadyToBuildLinkUnit
    );
    assert_eq!(
        plan.scheme_source,
        root.path().join(GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PATH)
    );
    assert_eq!(plan.header, header);
    assert!(
        plan.generated_runtime_scm
            .ends_with("deck-runtime-native~0.scm")
    );
    assert!(plan.object.ends_with("deck-runtime-native~0.o"));
    assert!(plan.link_c_source.ends_with("deck-runtime-native~0_.c"));
    assert!(plan.link_object.ends_with("deck-runtime-native~0_.o"));
    assert!(
        plan.static_scm
            .ends_with("marlin-deck-runtime__src__marlin__deck-runtime-native.scm")
    );
    assert_eq!(
        plan.exported_symbols
            .iter()
            .map(|symbol| symbol.as_str())
            .collect::<Vec<_>>(),
        [
            "marlin_deck_runtime_initialize",
            "marlin_deck_runtime_select_model_route"
        ]
    );
    assert_eq!(plan.gxc_generate_scheme.program, gxc);
    assert!(plan.gxc_generate_scheme.args.contains(&"-S".to_string()));
    assert!(plan.gxc_generate_scheme.args.contains(&"-s".to_string()));
    assert!(plan.gxc_generate_scheme.args.contains(&"-O".to_string()));
    assert_eq!(plan.gsc_compile_object.program, gsc);
    assert_eq!(
        plan.gsc_compile_object.args[..4],
        ["-target", "C", "-cc", "clang"]
    );
    assert!(plan.gsc_compile_object.args.contains(&"-obj".to_string()));
    assert_eq!(plan.gsc_generate_link_source.program, gsc);
    assert!(
        plan.gsc_generate_link_source
            .args
            .contains(&"-link".to_string())
    );
    assert!(
        !plan
            .gsc_generate_link_source
            .args
            .contains(&"-flat".to_string())
    );
    assert_eq!(plan.gsc_compile_link_object.program, gsc);
    assert!(
        plan.gsc_compile_link_object
            .args
            .contains(&"-obj".to_string())
    );
    assert!(
        plan.gsc_compile_link_object
            .args
            .contains(&"-cc-options".to_string())
    );
    assert!(
        plan.gsc_compile_link_object
            .args
            .contains(&"-D___LIBRARY".to_string())
    );
    assert!(
        plan.gsc_compile_link_object
            .args
            .iter()
            .any(|arg| arg.ends_with("deck-runtime-native~0_.c"))
    );
    assert_eq!(plan.audit_symbols.program, std::path::PathBuf::from("nm"));
    assert_eq!(
        plan.audit_symbols.args,
        [
            plan.object.to_string_lossy().into_owned(),
            plan.link_object.to_string_lossy().into_owned()
        ]
    );
    assert_eq!(plan.detail, None);
}

#[test]
fn deck_runtime_native_aot_plan_reports_missing_scheme_source() {
    let root = Builder::new()
        .prefix("marlin-gerbil-native-aot-missing-source-")
        .tempdir()
        .expect("create root");
    let gxc = root.path().join("toolchain/gxc");
    let gsc = root.path().join("toolchain/gsc");
    let header = root.path().join("include/marlin_deck_runtime_native.h");
    write_empty_file(&gxc);
    write_empty_file(&gsc);
    write_empty_file(&header);

    let plan = GerbilDeckRuntimeNativeAotConfig::new(root.path())
        .with_gxc(gxc)
        .with_gsc(gsc)
        .with_header(header)
        .plan();

    assert_eq!(
        plan.status,
        GerbilDeckRuntimeNativeAotStatus::MissingSchemeSource
    );
    assert!(
        plan.detail
            .as_deref()
            .is_some_and(|detail| detail.contains("missing native Deck runtime Scheme source"))
    );
}

#[test]
#[cfg(unix)]
fn deck_runtime_native_aot_build_runs_link_unit_runner() {
    let root = Builder::new()
        .prefix("marlin-gerbil-native-aot-build-")
        .tempdir()
        .expect("create root");
    let gerbil_prefix = root.path().join("gerbil-prefix");
    fs::create_dir_all(gerbil_prefix.join("bin")).expect("create Gerbil bin");
    fs::create_dir_all(gerbil_prefix.join("lib/gerbil")).expect("create Gerbil lib");
    write_empty_file(&gerbil_prefix.join("include/gambit.h"));
    write_empty_file(&gerbil_prefix.join("lib/libgambit.a"));

    let gxc = gerbil_prefix.join("bin/gxc");
    let gsc = gerbil_prefix.join("bin/gsc");
    let nm = root.path().join("toolchain/nm");
    let expected_prefix = gerbil_prefix.to_string_lossy();
    write_executable(
        &gxc,
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
out=""
previous=""
for arg in "$@"; do
  if [ "$previous" = "-d" ]; then
    out="$arg"
  fi
  previous="$arg"
done
mkdir -p "$out/marlin-deck-runtime/src/marlin" "$out/static"
: > "$out/marlin-deck-runtime/src/marlin/deck-runtime-native.scm"
: > "$out/marlin-deck-runtime/src/marlin/deck-runtime-native.ssi"
: > "$out/marlin-deck-runtime/src/marlin/deck-runtime-native.ssxi.ss"
: > "$out/marlin-deck-runtime/src/marlin/deck-runtime-native~0.scm"
: > "$out/static/marlin-deck-runtime__src__marlin__deck-runtime-native.scm"
"#,
            expected_prefix = expected_prefix
        )
        .as_str(),
    );
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
if [ "$mode" = "-link" ]; then
  : > "$dir/deck-runtime-native~0_.c"
elif [ "$base" = "deck-runtime-native~0_.c" ]; then
  : > "$dir/deck-runtime-native~0_.o"
else
  : > "$dir/deck-runtime-native~0.o"
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
        .with_gxc(gxc)
        .with_gsc(gsc)
        .with_c_compiler("clang")
        .with_symbol_auditor(nm)
        .with_gambit_link_search_dir(root.path().join("lib"))
        .build_link_unit();

    assert_eq!(
        receipt.status,
        GerbilDeckRuntimeNativeAotBuildStatus::LinkUnitReady
    );
    assert!(receipt.plan.scheme_source.is_file());
    assert!(receipt.plan.generated_runtime_scm.is_file());
    assert!(receipt.plan.object.is_file());
    assert!(receipt.plan.link_c_source.is_file());
    assert!(receipt.plan.link_object.is_file());
    assert_eq!(
        receipt
            .gxc_generate_scheme
            .as_ref()
            .and_then(|command| command.status_code),
        Some(0)
    );
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
fn deck_runtime_native_aot_build_rejects_missing_link_source() {
    let root = Builder::new()
        .prefix("marlin-gerbil-native-aot-missing-link-source-")
        .tempdir()
        .expect("create root");
    let gxc = root.path().join("toolchain/gxc");
    let gsc = root.path().join("toolchain/gsc");
    write_executable(
        &gxc,
        r#"#!/bin/sh
set -eu
out=""
previous=""
for arg in "$@"; do
  if [ "$previous" = "-d" ]; then
    out="$arg"
  fi
  previous="$arg"
done
mkdir -p "$out/marlin-deck-runtime/src/marlin" "$out/static"
: > "$out/marlin-deck-runtime/src/marlin/deck-runtime-native~0.scm"
"#,
    );
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
if [ "$mode" = "-obj" ]; then
  : > "$dir/deck-runtime-native~0.o"
fi
"#,
    );

    let receipt = GerbilDeckRuntimeNativeAotConfig::new(root.path())
        .with_gxc(gxc)
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
    let gxc = root.path().join("toolchain/gxc");
    let gsc = root.path().join("toolchain/gsc");
    write_executable(
        &gxc,
        r#"#!/bin/sh
set -eu
out=""
previous=""
for arg in "$@"; do
  if [ "$previous" = "-d" ]; then
    out="$arg"
  fi
  previous="$arg"
done
mkdir -p "$out/marlin-deck-runtime/src/marlin" "$out/static"
: > "$out/marlin-deck-runtime/src/marlin/deck-runtime-native~0.scm"
"#,
    );
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
if [ "$mode" = "-link" ]; then
  : > "$dir/deck-runtime-native~0_.c"
elif [ "$base" != "deck-runtime-native~0_.c" ]; then
  : > "$dir/deck-runtime-native~0.o"
fi
"#,
    );

    let receipt = GerbilDeckRuntimeNativeAotConfig::new(root.path())
        .with_gxc(gxc)
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
    let gxc = root.path().join("toolchain/gxc");
    let gsc = root.path().join("toolchain/gsc");
    let nm = root.path().join("toolchain/nm");
    write_executable(
        &gxc,
        r#"#!/bin/sh
set -eu
out=""
previous=""
for arg in "$@"; do
  if [ "$previous" = "-d" ]; then
    out="$arg"
  fi
  previous="$arg"
done
mkdir -p "$out/marlin-deck-runtime/src/marlin" "$out/static"
: > "$out/marlin-deck-runtime/src/marlin/deck-runtime-native~0.scm"
"#,
    );
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
if [ "$mode" = "-link" ]; then
  : > "$dir/deck-runtime-native~0_.c"
elif [ "$base" = "deck-runtime-native~0_.c" ]; then
  : > "$dir/deck-runtime-native~0_.o"
else
  : > "$dir/deck-runtime-native~0.o"
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
        .with_gxc(gxc)
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

#[test]
fn deck_runtime_native_static_link_plan_rejects_non_ready_object_receipt() {
    let root = Builder::new()
        .prefix("marlin-gerbil-native-link-not-ready-")
        .tempdir()
        .expect("create root");

    let receipt = GerbilDeckRuntimeNativeAotConfig::new(root.path())
        .with_gxc(root.path().join("missing-gxc"))
        .build_link_unit();
    let link_plan = receipt.static_link_plan();

    assert_eq!(
        link_plan.status,
        GerbilDeckRuntimeNativeStaticLinkStatus::LinkUnitNotReady
    );
    assert!(link_plan.cargo_directives.is_empty());
    assert!(
        link_plan
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("native Deck runtime link unit is not ready"))
    );
}

fn write_empty_file(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, "").expect("write file");
}

#[cfg(unix)]
fn write_executable(path: &Path, source: &str) {
    use std::os::unix::fs::PermissionsExt;

    write_empty_file(path);
    fs::write(path, source).expect("write executable");
    let mut permissions = fs::metadata(path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("set executable permissions");
}
