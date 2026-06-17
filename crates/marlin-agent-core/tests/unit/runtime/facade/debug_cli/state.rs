use marlin_agent_core::run_marlin_cli_from_args;
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn debug_cli_state_init_creates_unified_runtime_home_directories() {
    let dir = tempdir().expect("tempdir");
    let home = dir.path().join("runtime-home");

    let result = run_marlin_cli_from_args([
        "state",
        "init",
        "--home",
        home.to_str().expect("utf8 runtime home"),
    ]);

    assert_eq!(result.status, 0, "{}", result.stderr);
    let receipt: Value = serde_json::from_str(&result.stdout).expect("state init receipt");
    assert_eq!(receipt["status"], "Initialized");
    assert_eq!(receipt["home"]["path"], home.to_str().expect("utf8 home"));
    assert!(home.join("config").is_dir());
    assert!(home.join("cache").is_dir());
    assert!(home.join("cache").join("graph").is_dir());
    assert!(home.join("sessions").is_dir());
    assert!(home.join("memory").is_dir());
    assert!(home.join("receipts").is_dir());
    assert!(receipt.get("state_file_body").is_none());
}

#[test]
fn debug_cli_state_inspect_reports_directories_and_object_path_previews() {
    let dir = tempdir().expect("tempdir");
    let home = dir.path().join("runtime-home");

    let before = run_marlin_cli_from_args([
        "state",
        "inspect",
        "--home",
        home.to_str().expect("utf8 runtime home"),
    ]);
    assert_eq!(before.status, 0, "{}", before.stderr);
    let before_receipt: Value =
        serde_json::from_str(&before.stdout).expect("state inspect receipt");
    assert_eq!(
        before_receipt["home"]["path"],
        home.to_str().expect("utf8 home")
    );
    assert_eq!(
        before_receipt["object_paths"]["session"]["path"],
        home.join("sessions")
            .join("default-session.json")
            .to_str()
            .expect("utf8 path")
    );
    assert_eq!(
        before_receipt["object_paths"]["memory_shard"]["path"],
        home.join("memory")
            .join("default-memory-shard.json")
            .to_str()
            .expect("utf8 path")
    );
    assert_eq!(
        before_receipt["object_paths"]["graph_cache"]["path"],
        home.join("cache")
            .join("graph")
            .join("default-graph-cache.json")
            .to_str()
            .expect("utf8 path")
    );
    assert_eq!(before_receipt["directories"][0]["exists"], false);
    assert!(before_receipt.get("state_file_body").is_none());

    let init = run_marlin_cli_from_args([
        "state",
        "init",
        "--home",
        home.to_str().expect("utf8 runtime home"),
    ]);
    assert_eq!(init.status, 0, "{}", init.stderr);

    let after = run_marlin_cli_from_args([
        "state",
        "inspect",
        "--home",
        home.to_str().expect("utf8 runtime home"),
    ]);
    assert_eq!(after.status, 0, "{}", after.stderr);
    let after_receipt: Value = serde_json::from_str(&after.stdout).expect("state inspect receipt");
    assert!(
        after_receipt["directories"]
            .as_array()
            .expect("directories array")
            .iter()
            .all(|directory| directory["exists"] == true && directory["is_dir"] == true)
    );
}
