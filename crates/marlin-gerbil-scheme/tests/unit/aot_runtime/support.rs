use std::{fs, path::Path};

pub(super) fn write_empty_file(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, "").expect("write file");
}

#[cfg(unix)]
pub(super) fn write_executable(path: &Path, source: &str) {
    use std::os::unix::fs::PermissionsExt;

    write_empty_file(path);
    fs::write(path, source).expect("write executable");
    let mut permissions = fs::metadata(path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("set executable permissions");
}
