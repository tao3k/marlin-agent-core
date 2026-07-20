use marlin_gerbil_scheme::{
    GERBIL_LOADPATH_ENV, default_gerbil_gxi_program, gerbil_package_root,
    gerbil_runtime_dependency_loadpath, gerbil_runtime_loadpath_with_dependencies,
};
use std::{
    io,
    process::Command,
    time::{Duration, Instant},
};

const POLICY_BRIDGE_COLD_LOAD_MAX: Duration = Duration::from_secs(45);

#[test]
fn gerbil_policy_bridge_load_stays_under_budget_when_gxi_available() {
    let package_root = gerbil_package_root();
    let dependency_loadpath = gerbil_runtime_dependency_loadpath();
    if !dependency_loadpath.exists() {
        eprintln!(
            "skipping Gerbil policy bridge performance gate: package-local dependency loadpath is absent: {}",
            dependency_loadpath.display()
        );
        return;
    }

    let gxi = default_gerbil_gxi_program();
    let started_at = Instant::now();
    let output = match Command::new(&gxi)
        .current_dir(&package_root)
        .env(
            GERBIL_LOADPATH_ENV,
            gerbil_runtime_loadpath_with_dependencies(&package_root),
        )
        .arg("-e")
        .arg("(import :gslph/src/policy/facade)")
        .arg("-e")
        .arg("(display \"gslph-policy-facade-ready\")")
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            eprintln!(
                "skipping Gerbil policy bridge performance gate: gxi executable not found: {}",
                gxi.display()
            );
            return;
        }
        Err(error) => panic!("run Gerbil policy bridge load gate: {error}"),
    };
    let elapsed = started_at.elapsed();

    assert!(
        output.status.success(),
        "Gerbil policy bridge load failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("gslph-policy-facade-ready"),
        "Gerbil policy facade did not emit its readiness receipt\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        elapsed <= POLICY_BRIDGE_COLD_LOAD_MAX,
        "Gerbil policy bridge cold load exceeded budget: elapsed={elapsed:?} max={POLICY_BRIDGE_COLD_LOAD_MAX:?}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
