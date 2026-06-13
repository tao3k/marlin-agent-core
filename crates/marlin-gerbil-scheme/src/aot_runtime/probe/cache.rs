//! Persistent cache for unavailable Gerbil AOT toolchain states.

use super::{
    config::GerbilAotProbeConfig,
    constants::{GERBIL_AOT_EXECUTABLE_NAME, GERBIL_AOT_PROBE_CACHE_SCHEMA_VERSION},
    receipt::GerbilAotProbeReceipt,
    status::GerbilAotProbeStatus,
};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path, path::PathBuf};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct GerbilAotProbeCacheRecord {
    schema_version: u32,
    status: GerbilAotProbeStatus,
    gxc: PathBuf,
    gsc: PathBuf,
    backend_gsc: Option<PathBuf>,
}

pub(super) fn read_gerbil_aot_probe_cache(
    cache_path: &Path,
    config: &GerbilAotProbeConfig,
) -> Option<GerbilAotProbeReceipt> {
    let source = fs::read_to_string(cache_path).ok()?;
    let record = serde_json::from_str::<GerbilAotProbeCacheRecord>(&source).ok()?;
    if !record.is_valid_for(config) {
        return None;
    }
    Some(record.to_receipt(config, cache_path))
}

pub(super) fn write_gerbil_aot_probe_cache(
    cache_path: &Path,
    receipt: &GerbilAotProbeReceipt,
) -> io::Result<()> {
    if !is_cacheable_gerbil_aot_probe_receipt(receipt) {
        return Ok(());
    }
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let record = GerbilAotProbeCacheRecord::from_receipt(receipt);
    let payload = serde_json::to_vec_pretty(&record)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    fs::write(cache_path, payload)
}

impl GerbilAotProbeCacheRecord {
    fn from_receipt(receipt: &GerbilAotProbeReceipt) -> Self {
        Self {
            schema_version: GERBIL_AOT_PROBE_CACHE_SCHEMA_VERSION,
            status: receipt.status,
            gxc: receipt.gxc.clone(),
            gsc: receipt.gsc.clone(),
            backend_gsc: receipt.backend_gsc.clone(),
        }
    }

    fn is_valid_for(&self, config: &GerbilAotProbeConfig) -> bool {
        if self.schema_version != GERBIL_AOT_PROBE_CACHE_SCHEMA_VERSION
            || self.gxc != config.gxc
            || self.gsc != config.gsc
        {
            return false;
        }

        match self.status {
            GerbilAotProbeStatus::MissingGxc => !config.gxc.is_file(),
            GerbilAotProbeStatus::MissingGsc => config.gxc.is_file() && !config.gsc.is_file(),
            GerbilAotProbeStatus::GscBackendUnavailable => {
                config.gxc.is_file()
                    && config.gsc.is_file()
                    && self
                        .backend_gsc
                        .as_ref()
                        .is_some_and(|path| !path.is_file())
            }
            GerbilAotProbeStatus::AssetWriteFailed
            | GerbilAotProbeStatus::ModuleCompileFailed
            | GerbilAotProbeStatus::ExecutableCompileFailed
            | GerbilAotProbeStatus::ExecutableReady => false,
        }
    }

    fn to_receipt(
        &self,
        config: &GerbilAotProbeConfig,
        cache_path: &Path,
    ) -> GerbilAotProbeReceipt {
        GerbilAotProbeReceipt {
            status: self.status,
            gxc: config.gxc.clone(),
            gsc: config.gsc.clone(),
            backend_gsc: self.backend_gsc.clone(),
            root: config.root.clone(),
            executable: config.root.join(GERBIL_AOT_EXECUTABLE_NAME),
            detail: Some(format!(
                "cached Gerbil AOT toolchain status from {}",
                cache_path.display()
            )),
            module_compile: None,
            executable_compile: None,
        }
    }
}

fn is_cacheable_gerbil_aot_probe_receipt(receipt: &GerbilAotProbeReceipt) -> bool {
    match receipt.status {
        GerbilAotProbeStatus::MissingGxc | GerbilAotProbeStatus::MissingGsc => true,
        GerbilAotProbeStatus::GscBackendUnavailable => receipt.backend_gsc.is_some(),
        GerbilAotProbeStatus::AssetWriteFailed
        | GerbilAotProbeStatus::ModuleCompileFailed
        | GerbilAotProbeStatus::ExecutableCompileFailed
        | GerbilAotProbeStatus::ExecutableReady => false,
    }
}
