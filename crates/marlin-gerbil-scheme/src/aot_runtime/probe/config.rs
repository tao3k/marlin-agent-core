//! Configuration type for Gerbil AOT probing.

use std::path::PathBuf;

/// Configuration for a `Gerbil` ahead-of-time runtime compiler probe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilAotProbeConfig {
    pub(super) root: PathBuf,
    pub(super) gxc: PathBuf,
    pub(super) gsc: PathBuf,
}
