use marlin_gerbil_scheme::GerbilResidentRuntimePlan;
use tempfile::Builder;

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn resident_runtime_real_gxi_batch_launcher_starts_and_terminates() {
    let root = Builder::new()
        .prefix("marlin-resident-runtime-real-gxi-")
        .tempdir()
        .expect("resident runtime tempdir");
    let handle = GerbilResidentRuntimePlan::shared_context(root.path(), "real-gxi-session")
        .prepare()
        .expect("prepare resident runtime");

    let mut process = handle
        .spawn_process()
        .expect("spawn resident runtime batch adapter");

    assert!(process.child_id() > 0);
    assert!(process.is_running().expect("resident runtime status"));
    let _ = process
        .terminate()
        .expect("terminate resident runtime batch adapter");
}
