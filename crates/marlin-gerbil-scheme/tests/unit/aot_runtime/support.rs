use std::{fs, path::Path};

pub(super) fn write_empty_file(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, "").expect("write file");
}

pub(super) fn write_deck_runtime_native_aot_dependency_scms(root: &Path) {
    write_empty_file(&root.join(".gerbil/native/_deck-runtime-native~0.scm"));
}

pub(super) fn write_agent_policy_routing_native_aot_dependency_scms(root: &Path) {
    write_empty_file(&root.join(".gerbil/native/_agent-policy-routing-native~0.scm"));
}

pub(super) fn write_deck_runtime_native_aot_scms(root: &Path) -> std::path::PathBuf {
    write_deck_runtime_native_aot_dependency_scms(root);
    let compiled_runtime_scm = root.join(".gerbil/native/deck-runtime-native~0.scm");
    write_empty_file(&compiled_runtime_scm);
    compiled_runtime_scm
}

pub(super) fn write_agent_policy_routing_native_aot_scms(root: &Path) -> std::path::PathBuf {
    write_agent_policy_routing_native_aot_dependency_scms(root);
    let compiled_runtime_scm = root.join(".gerbil/native/agent-policy-routing-native~0.scm");
    write_empty_file(&compiled_runtime_scm);
    compiled_runtime_scm
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
