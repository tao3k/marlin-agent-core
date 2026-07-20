use std::path::PathBuf;

const STORAGE_PACKAGE_PATH: &str = "crates/marlin-agent-storage";
#[cfg(feature = "turso")]
pub(crate) const STORAGE_PACKAGE_NAME: &str = "marlin-agent-storage";

fn bazel_workspace_root() -> Option<PathBuf> {
    let runfiles_root =
        std::env::var_os("TEST_SRCDIR").or_else(|| std::env::var_os("RUNFILES_DIR"))?;
    let workspace = std::env::var_os("TEST_WORKSPACE")?;
    let root = PathBuf::from(runfiles_root).join(workspace);
    root.is_dir().then_some(root)
}

pub(crate) fn crate_root() -> PathBuf {
    bazel_workspace_root()
        .map(|root| root.join(STORAGE_PACKAGE_PATH))
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

pub(crate) fn workspace_root() -> PathBuf {
    bazel_workspace_root().unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("storage crate should live under <workspace>/crates")
            .to_path_buf()
    })
}
