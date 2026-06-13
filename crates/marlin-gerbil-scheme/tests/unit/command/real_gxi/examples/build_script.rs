use super::support::test_root;
use super::support::{MARLIN_REQUIRE_REAL_GXI_ENV, local_gxi};
use marlin_gerbil_scheme::{
    GERBIL_LOADPATH_ENV, GerbilDeckRuntimeNativeAotBuildStatus, GerbilDeckRuntimeNativeAotConfig,
    GerbilDeckRuntimeNativeStaticLinkStatus, gerbil_runtime_loadpath, write_gerbil_runtime_assets,
};
use std::process::Command;

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_build_script_compiles_runtime_assets() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = test_root("runtime-build-script");
    write_gerbil_runtime_assets(root.path()).expect("write gerbil runtime assets");

    let output = Command::new(gxi)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(root.path()))
        .current_dir(root.path())
        .arg(root.path().join("build.ss"))
        .arg("compile")
        .output()
        .expect("run real gxi build script");

    assert!(
        output.status.success(),
        "gxi build script failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "requires a local Gerbil gxc/gsc toolchain and C compiler"]
fn command_compiler_real_gxi_deck_runtime_native_compiles_link_unit() {
    if local_gxi().is_none() {
        return;
    }
    let root = test_root("runtime-native-link-unit");
    let config = GerbilDeckRuntimeNativeAotConfig::new(root.path());
    let config = if cfg!(target_os = "macos") {
        config.with_c_compiler("clang")
    } else {
        config
    };
    let receipt = config.build_link_unit();
    if receipt.status != GerbilDeckRuntimeNativeAotBuildStatus::LinkUnitReady {
        let message = format!("native AOT link unit build failed: {receipt:?}");
        if std::env::var_os(MARLIN_REQUIRE_REAL_GXI_ENV).is_some() {
            panic!("{message}");
        }
        eprintln!("{message}");
        return;
    }

    assert!(
        receipt.plan.generated_runtime_scm.is_file(),
        "missing generated runtime Scheme at {}",
        receipt.plan.generated_runtime_scm.display()
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
}
