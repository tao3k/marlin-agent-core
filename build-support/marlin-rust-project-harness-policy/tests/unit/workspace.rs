use std::{
    env,
    path::{Path, PathBuf},
    sync::OnceLock,
};

const POLICY_PACKAGE: &str = "build-support/marlin-rust-project-harness-policy";

static WORKSPACE_ROOT: OnceLock<PathBuf> = OnceLock::new();
static POLICY_PACKAGE_ROOT: OnceLock<PathBuf> = OnceLock::new();

pub(crate) fn workspace_root() -> &'static Path {
    WORKSPACE_ROOT.get_or_init(resolve_workspace_root).as_path()
}

pub(crate) fn policy_package_root() -> &'static Path {
    POLICY_PACKAGE_ROOT
        .get_or_init(|| workspace_root().join(POLICY_PACKAGE))
        .as_path()
}

fn resolve_workspace_root() -> PathBuf {
    let test_workspace = env::var_os("TEST_WORKSPACE");

    for runfiles_root in [env::var_os("TEST_SRCDIR"), env::var_os("RUNFILES_DIR")]
        .into_iter()
        .flatten()
    {
        if let Some(test_workspace) = &test_workspace {
            let candidate = PathBuf::from(runfiles_root).join(test_workspace);
            if candidate.join("Cargo.toml").is_file() {
                return candidate;
            }
        }
    }

    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("policy crate should live under workspace/build-support")
        .to_path_buf()
}
