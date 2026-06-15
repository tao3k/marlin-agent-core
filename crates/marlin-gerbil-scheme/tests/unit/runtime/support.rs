use marlin_gerbil_scheme::{GerbilAotProbeReceipt, GerbilAotProbeStatus};
use std::{
    fs,
    ops::Deref,
    path::{Path, PathBuf},
};
use tempfile::{Builder, TempDir};

pub(super) struct TestRoot(TempDir);

impl TestRoot {
    pub(super) fn path(&self) -> &Path {
        self.0.path()
    }
}

impl Deref for TestRoot {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.path()
    }
}

impl AsRef<Path> for TestRoot {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}

impl From<&TestRoot> for PathBuf {
    fn from(root: &TestRoot) -> Self {
        root.path().to_path_buf()
    }
}

pub(super) fn test_root(name: &str) -> TestRoot {
    TestRoot(
        Builder::new()
            .prefix(&format!("marlin-gerbil-scheme-{name}-"))
            .tempdir()
            .unwrap_or_else(|error| panic!("creates {name} test root: {error}")),
    )
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
        executable: root
            .join("probe-root")
            .join("marlin-gerbil-typed-runtime-aot"),
        detail: None,
        module_compile: None,
        executable_compile: None,
    }
}

#[cfg(unix)]
pub(super) fn line_count(path: &Path) -> usize {
    fs::read_to_string(path).unwrap_or_default().lines().count()
}
