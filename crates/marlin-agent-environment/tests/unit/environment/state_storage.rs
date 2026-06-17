use std::{
    fs,
    path::PathBuf,
    process,
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_agent_environment::RuntimeStateStorageInitializer;
use marlin_agent_protocol::{RuntimeHome, RuntimeStateDirectoryKind, RuntimeStateStorageStatus};

#[test]
fn state_storage_initializer_creates_standard_runtime_home_directories() {
    let temp_root = unique_temp_root("marlin-state-home");
    let home = RuntimeHome::default_for_user_home(&temp_root);
    let layout = home.state_layout();

    let receipt = RuntimeStateStorageInitializer::new().ensure_layout(&layout);

    assert_eq!(receipt.status, RuntimeStateStorageStatus::Initialized);
    assert!(is_dir(layout.path_for(RuntimeStateDirectoryKind::Config)));
    assert!(is_dir(layout.path_for(RuntimeStateDirectoryKind::Cache)));
    assert!(is_dir(
        layout.path_for(RuntimeStateDirectoryKind::GraphCache)
    ));
    assert!(is_dir(layout.path_for(RuntimeStateDirectoryKind::Sessions)));
    assert!(is_dir(layout.path_for(RuntimeStateDirectoryKind::Memory)));
    assert!(is_dir(layout.path_for(RuntimeStateDirectoryKind::Receipts)));
    assert_eq!(receipt.directories.len(), layout.directories.len());
    assert!(receipt.reason.is_none());

    fs::remove_dir_all(temp_root).expect("temporary state home should be removable");
}

fn is_dir(path: Option<&PathBuf>) -> bool {
    path.map(|path| path.is_dir()).unwrap_or(false)
}

fn unique_temp_root(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", process::id()))
}
