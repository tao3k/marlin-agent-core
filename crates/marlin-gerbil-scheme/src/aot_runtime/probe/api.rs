//! Public method implementations for Gerbil AOT probe configuration.

use super::{
    cache::{read_gerbil_aot_probe_cache, write_gerbil_aot_probe_cache},
    config::GerbilAotProbeConfig,
    receipt::GerbilAotProbeReceipt,
    run::run_gerbil_aot_probe,
};
use crate::runtime::{default_gerbil_gsc_program, default_gerbil_gxc_program};
use std::{
    io,
    path::{Path, PathBuf},
};

impl GerbilAotProbeConfig {
    /// Builds a probe rooted at a writable runtime asset directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            gxc: default_gerbil_gxc_program(),
            gsc: default_gerbil_gsc_program(),
        }
    }

    /// Overrides the `gxc` executable used by the probe.
    pub fn with_gxc(mut self, gxc: impl Into<PathBuf>) -> Self {
        self.gxc = gxc.into();
        self
    }

    /// Overrides the paired `Gerbil` Gambit compiler used by the probe.
    pub fn with_gsc(mut self, gsc: impl Into<PathBuf>) -> Self {
        self.gsc = gsc.into();
        self
    }

    /// Runs the probe and returns a typed receipt instead of panicking.
    pub fn probe(&self) -> GerbilAotProbeReceipt {
        run_gerbil_aot_probe(self)
    }

    /// Runs the probe with a persistent cache for unavailable toolchain states.
    pub fn probe_with_toolchain_cache(
        &self,
        cache_path: impl AsRef<Path>,
    ) -> io::Result<GerbilAotProbeReceipt> {
        let cache_path = cache_path.as_ref();
        if let Some(receipt) = read_gerbil_aot_probe_cache(cache_path, self) {
            return Ok(receipt);
        }

        let receipt = self.probe();
        write_gerbil_aot_probe_cache(cache_path, &receipt)?;
        Ok(receipt)
    }
}
