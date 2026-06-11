use marlin_gerbil_scheme::{GerbilAotProbeReceipt, GerbilAotProbeStatus};
use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) fn test_root(name: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "marlin-gerbil-scheme-{name}-{}-{suffix}",
        std::process::id()
    ))
}

pub(super) fn gerbil_backend_failure_receipt(
    root: &Path,
    gsc: &Path,
    backend_gsc: &Path,
) -> GerbilAotProbeReceipt {
    GerbilAotProbeReceipt {
        status: GerbilAotProbeStatus::GscBackendUnavailable,
        gxc: root.join("gxc"),
        gsc: gsc.to_path_buf(),
        backend_gsc: Some(backend_gsc.to_path_buf()),
        root: root.join("probe-root"),
        executable: root.join("probe-root").join("command-adapter-aot"),
        detail: None,
        module_compile: None,
        executable_compile: None,
    }
}

#[cfg(unix)]
pub(super) fn line_count(path: &Path) -> usize {
    fs::read_to_string(path).unwrap_or_default().lines().count()
}
