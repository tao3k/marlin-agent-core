use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{atomic_write_with_parent_sync, write_new_temp};

#[test]
fn existing_temporary_path_is_never_overwritten() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("marlin-org-temp-{}-{unique}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let path = root.join("temporary");
    fs::write(&path, "existing").unwrap();
    assert!(write_new_temp(&path, "new content").is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), "existing");
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn temporary_symlink_is_not_followed() {
    use std::os::unix::fs::symlink;

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("marlin-org-link-{}-{unique}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let target = root.join("target");
    let path = root.join("temporary");
    fs::write(&target, "existing").unwrap();
    symlink(&target, &path).unwrap();
    assert!(write_new_temp(&path, "new content").is_err());
    assert_eq!(fs::read_to_string(&target).unwrap(), "existing");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_parent_sync_reports_uncertain_after_replacement() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("marlin-org-sync-{}-{unique}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let path = root.join("document.org");
    fs::write(&path, "old").unwrap();
    let result = atomic_write_with_parent_sync(&path, "new", |_| {
        Err(std::io::Error::other("injected sync failure"))
    });
    assert!(result.is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), "new");
    fs::remove_dir_all(root).unwrap();
}
