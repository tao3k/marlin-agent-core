//! Shared filesystem paths for `scenario_benchmark` gates.

use std::path::{Path, PathBuf};

pub(super) fn scenario_root(crate_root: &Path) -> PathBuf {
    crate_root
        .join("tests")
        .join("unit")
        .join("scenarios")
        .join("performance_baseline")
}
