use super::support::{MARLIN_REQUIRE_REAL_GXI_ENV, local_gxi};
use marlin_gerbil_scheme::{GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
#[ignore = "requires local Gerbil gxi/gxtest executables and installed gerbil-poo dependency"]
fn command_compiler_real_gxi_runs_all_runtime_gxtests() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let Some(gxtest) = local_gxtest_for_gxi(&gxi) else {
        return;
    };
    let gerbil_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("gerbil");
    let test_dir = gerbil_root.join("t");
    let mut tests = fs::read_dir(&test_dir)
        .expect("read Gerbil runtime test directory")
        .map(|entry| entry.expect("read Gerbil runtime test entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ss"))
        .collect::<Vec<_>>();
    tests.sort();

    assert!(
        tests
            .iter()
            .any(|path| path.ends_with("deck-runtime-native-projection-test.ss")),
        "runtime gxtest suite should include the native projection contract test"
    );

    for test in tests {
        let output = Command::new(&gxtest)
            .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(&gerbil_root))
            .current_dir(&gerbil_root)
            .arg(&test)
            .output()
            .unwrap_or_else(|error| panic!("run gxtest {}: {error}", test.display()));

        assert!(
            output.status.success(),
            "gxtest {} failed\nstdout:\n{}\nstderr:\n{}",
            test.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn local_gxtest_for_gxi(gxi: &Path) -> Option<PathBuf> {
    let Some(parent) = gxi.parent() else {
        return missing_gxtest(gxi.with_file_name("gxtest"));
    };
    let gxtest = parent.join("gxtest");
    if gxtest.exists() {
        return Some(gxtest);
    }
    missing_gxtest(gxtest)
}

fn missing_gxtest(gxtest: PathBuf) -> Option<PathBuf> {
    let message = format!(
        "skipping real gxtest because {} is missing",
        gxtest.display()
    );
    if std::env::var_os(MARLIN_REQUIRE_REAL_GXI_ENV).is_some() {
        panic!("{message}; unset {MARLIN_REQUIRE_REAL_GXI_ENV} or install matching gxtest");
    }
    eprintln!("{message}");
    None
}
