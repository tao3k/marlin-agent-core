//! Backend `gsc` repair planning and local shim creation.

use super::{
    receipt::{GerbilAotBackendRepairReceipt, GerbilAotBackendShimReceipt, GerbilAotProbeReceipt},
    status::{GerbilAotBackendRepairStatus, GerbilAotBackendShimStatus, GerbilAotProbeStatus},
};
use std::{fs, io, path::Path, path::PathBuf};

impl GerbilAotProbeReceipt {
    /// Plans a backend `gsc` repair without writing files.
    pub fn plan_backend_gsc_repair(
        &self,
        allowed_root: impl AsRef<Path>,
    ) -> io::Result<GerbilAotBackendRepairReceipt> {
        plan_gerbil_aot_backend_gsc_repair(self, allowed_root.as_ref())
    }

    /// Prepares a backend `gsc` shim only when the target path is inside `allowed_root`.
    pub fn prepare_backend_gsc_shim(
        &self,
        allowed_root: impl AsRef<Path>,
    ) -> io::Result<GerbilAotBackendShimReceipt> {
        prepare_gerbil_aot_backend_gsc_shim(self, allowed_root.as_ref())
    }
}

pub(super) fn plan_gerbil_aot_backend_gsc_repair(
    receipt: &GerbilAotProbeReceipt,
    allowed_root: &Path,
) -> io::Result<GerbilAotBackendRepairReceipt> {
    if receipt.status != GerbilAotProbeStatus::GscBackendUnavailable {
        return Ok(gerbil_aot_backend_repair_receipt(
            receipt,
            allowed_root,
            GerbilAotBackendRepairStatus::NotNeeded,
            false,
            false,
            "leave toolchain unchanged",
            Some("AOT probe did not report a backend gsc failure".to_owned()),
        ));
    }
    let Some(backend_gsc) = receipt.backend_gsc.as_ref() else {
        return Ok(gerbil_aot_backend_repair_receipt(
            receipt,
            allowed_root,
            GerbilAotBackendRepairStatus::MissingBackendPath,
            false,
            false,
            "rerun probe and capture backend gsc path",
            Some("AOT probe did not expose the backend gsc path".to_owned()),
        ));
    };
    if !receipt.gsc.is_file() {
        return Ok(gerbil_aot_backend_repair_receipt(
            receipt,
            allowed_root,
            GerbilAotBackendRepairStatus::MissingConfiguredGsc,
            false,
            false,
            "install or configure the Gerbil Gambit compiler",
            Some(format!(
                "configured gsc is missing at {}",
                receipt.gsc.display()
            )),
        ));
    }
    if backend_gsc.is_file() {
        return Ok(gerbil_aot_backend_repair_receipt(
            receipt,
            allowed_root,
            GerbilAotBackendRepairStatus::AlreadyReady,
            false,
            false,
            "rerun AOT probe",
            Some(format!(
                "backend gsc already exists at {}",
                backend_gsc.display()
            )),
        ));
    }

    if is_path_inside_allowed_root(backend_gsc, allowed_root)? {
        return Ok(gerbil_aot_backend_repair_receipt(
            receipt,
            allowed_root,
            GerbilAotBackendRepairStatus::RepoShimAvailable,
            true,
            false,
            "create backend gsc shim inside the allowed root",
            Some(format!(
                "backend gsc {} can be linked to configured gsc {}",
                backend_gsc.display(),
                receipt.gsc.display()
            )),
        ));
    }

    Ok(gerbil_aot_backend_repair_receipt(
        receipt,
        allowed_root,
        GerbilAotBackendRepairStatus::RequiresSystemWrite,
        false,
        true,
        "explicitly authorize a system-level shim or fix Gerbil/Gambit configuration",
        Some(format!(
            "backend gsc {} is outside allowed root {}",
            backend_gsc.display(),
            allowed_root.display()
        )),
    ))
}

pub(super) fn prepare_gerbil_aot_backend_gsc_shim(
    receipt: &GerbilAotProbeReceipt,
    allowed_root: &Path,
) -> io::Result<GerbilAotBackendShimReceipt> {
    if receipt.status != GerbilAotProbeStatus::GscBackendUnavailable {
        return Ok(gerbil_aot_backend_shim_receipt(
            receipt,
            allowed_root,
            GerbilAotBackendShimStatus::NotNeeded,
            Some("AOT probe did not report a backend gsc failure".to_owned()),
        ));
    }
    let Some(backend_gsc) = receipt.backend_gsc.as_ref() else {
        return Ok(gerbil_aot_backend_shim_receipt(
            receipt,
            allowed_root,
            GerbilAotBackendShimStatus::MissingBackendPath,
            Some("AOT probe did not expose the backend gsc path".to_owned()),
        ));
    };
    if !receipt.gsc.is_file() {
        return Ok(gerbil_aot_backend_shim_receipt(
            receipt,
            allowed_root,
            GerbilAotBackendShimStatus::MissingConfiguredGsc,
            Some(format!(
                "configured gsc is missing at {}",
                receipt.gsc.display()
            )),
        ));
    }
    if backend_gsc.is_file() {
        return Ok(gerbil_aot_backend_shim_receipt(
            receipt,
            allowed_root,
            GerbilAotBackendShimStatus::AlreadyReady,
            Some(format!(
                "backend gsc already exists at {}",
                backend_gsc.display()
            )),
        ));
    }
    if !is_path_inside_allowed_root(backend_gsc, allowed_root)? {
        return Ok(gerbil_aot_backend_shim_receipt(
            receipt,
            allowed_root,
            GerbilAotBackendShimStatus::OutsideAllowedRoot,
            Some(format!(
                "backend gsc {} is outside allowed root {}",
                backend_gsc.display(),
                allowed_root.display()
            )),
        ));
    }

    if let Some(parent) = backend_gsc.parent() {
        fs::create_dir_all(parent)?;
    }
    write_backend_gsc_shim(&receipt.gsc, backend_gsc)?;
    Ok(gerbil_aot_backend_shim_receipt(
        receipt,
        allowed_root,
        GerbilAotBackendShimStatus::Created,
        Some(format!(
            "created backend gsc shim {} -> {}",
            backend_gsc.display(),
            receipt.gsc.display()
        )),
    ))
}

fn gerbil_aot_backend_shim_receipt(
    receipt: &GerbilAotProbeReceipt,
    allowed_root: &Path,
    status: GerbilAotBackendShimStatus,
    detail: Option<String>,
) -> GerbilAotBackendShimReceipt {
    GerbilAotBackendShimReceipt {
        status,
        gsc: receipt.gsc.clone(),
        backend_gsc: receipt.backend_gsc.clone(),
        allowed_root: allowed_root.to_path_buf(),
        detail,
    }
}

fn gerbil_aot_backend_repair_receipt(
    receipt: &GerbilAotProbeReceipt,
    allowed_root: &Path,
    status: GerbilAotBackendRepairStatus,
    can_create_shim: bool,
    requires_system_write: bool,
    recommended_action: &str,
    detail: Option<String>,
) -> GerbilAotBackendRepairReceipt {
    GerbilAotBackendRepairReceipt {
        status,
        gsc: receipt.gsc.clone(),
        backend_gsc: receipt.backend_gsc.clone(),
        allowed_root: allowed_root.to_path_buf(),
        can_create_shim,
        requires_system_write,
        recommended_action: recommended_action.to_owned(),
        detail,
    }
}

fn is_path_inside_allowed_root(path: &Path, allowed_root: &Path) -> io::Result<bool> {
    let Some(allowed_root) = normalize_path_for_containment(allowed_root)? else {
        return Ok(false);
    };
    let Some(path) = normalize_path_for_containment(path)? else {
        return Ok(false);
    };
    Ok(path.starts_with(allowed_root))
}

fn normalize_path_for_containment(path: &Path) -> io::Result<Option<PathBuf>> {
    if path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Ok(None);
    }

    let existing = nearest_existing_ancestor(path)?;
    let mut normalized = fs::canonicalize(&existing)?;
    if let Ok(tail) = path.strip_prefix(&existing) {
        for component in tail.components() {
            match component {
                std::path::Component::Normal(segment) => normalized.push(segment),
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_) => return Ok(None),
            }
        }
    }
    Ok(Some(normalized))
}

fn nearest_existing_ancestor(path: &Path) -> io::Result<PathBuf> {
    let mut candidate = path.to_path_buf();
    loop {
        if candidate.exists() {
            return Ok(candidate);
        }
        if !candidate.pop() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no existing ancestor for {}", path.display()),
            ));
        }
    }
}

#[cfg(unix)]
fn write_backend_gsc_shim(source: &Path, destination: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(source, destination)
}

#[cfg(not(unix))]
fn write_backend_gsc_shim(source: &Path, destination: &Path) -> io::Result<()> {
    fs::copy(source, destination).map(|_| ())
}
