//! Runs the Rust project Harness once for every Cargo workspace package.
//!
//! This is the explicit, addressable replacement for package-local policy-only
//! build scripts. Cargo metadata owns workspace membership, while each member is
//! still checked through Marlin's strict package-scoped Harness adapter.

use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::assert_marlin_rust_project_harness_build_check_from_env;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Selects the Cargo workspace whose members must pass the package gate.
pub struct WorkspacePackageGateRequest {
    workspace_root: PathBuf,
}

impl WorkspacePackageGateRequest {
    /// Creates a request with a canonical workspace root.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Result<Self, String> {
        let workspace_root = workspace_root.into();
        let workspace_root = workspace_root.canonicalize().map_err(|error| {
            format!("cannot canonicalize {}: {error}", workspace_root.display())
        })?;
        Ok(Self { workspace_root })
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
/// Identifies one Cargo package covered by a workspace gate receipt.
pub struct WorkspacePackageName(String);

impl WorkspacePackageName {
    /// Returns the Cargo package name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Proves that every discovered workspace package completed the Harness gate.
pub struct WorkspacePackageGateReceipt {
    workspace_root: PathBuf,
    packages: Vec<WorkspacePackageName>,
}

impl WorkspacePackageGateReceipt {
    /// Returns the canonical workspace root used for discovery.
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Returns the packages that completed the gate.
    pub fn packages(&self) -> &[WorkspacePackageName] {
        &self.packages
    }
}

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
    workspace_members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    id: String,
    name: String,
    manifest_path: PathBuf,
}

/// Parses the package-gate command line, runs the gate, and prints its receipt.
pub fn run_workspace_package_gate_cli(args: impl Iterator<Item = OsString>) -> ExitCode {
    match parse_request(args).and_then(run_workspace_package_gate) {
        Ok(receipt) => {
            let package_names = receipt
                .packages()
                .iter()
                .map(WorkspacePackageName::as_str)
                .collect::<Vec<_>>()
                .join(",");
            println!(
                "schema=marlin.workspace-package-gate status=passed workspace_root={} package_count={} packages={package_names}",
                receipt.workspace_root().display(),
                receipt.packages().len(),
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("schema=marlin.workspace-package-gate status=failed error={error}");
            ExitCode::FAILURE
        }
    }
}

/// Runs Marlin's strict package-scoped Harness gate for every workspace member.
pub fn run_workspace_package_gate(
    request: WorkspacePackageGateRequest,
) -> Result<WorkspacePackageGateReceipt, String> {
    let metadata = load_cargo_metadata(&request.workspace_root)?;
    let workspace_members = metadata
        .workspace_members
        .into_iter()
        .collect::<HashSet<_>>();
    let mut packages = metadata
        .packages
        .into_iter()
        .filter(|package| workspace_members.contains(&package.id))
        .collect::<Vec<_>>();
    packages.sort_by(|left, right| {
        left.manifest_path
            .cmp(&right.manifest_path)
            .then_with(|| left.name.cmp(&right.name))
    });
    if packages.is_empty() {
        return Err(format!(
            "cargo metadata returned no workspace packages for {}",
            request.workspace_root.display()
        ));
    }

    let mut package_names = Vec::with_capacity(packages.len());
    for package in packages {
        let package_root = package.manifest_path.parent().ok_or_else(|| {
            format!(
                "workspace package {} has no manifest parent: {}",
                package.name,
                package.manifest_path.display()
            )
        })?;
        assert_marlin_rust_project_harness_build_check_from_env(package_root);
        package_names.push(WorkspacePackageName(package.name));
    }

    Ok(WorkspacePackageGateReceipt {
        workspace_root: request.workspace_root,
        packages: package_names,
    })
}

fn parse_request(
    args: impl Iterator<Item = OsString>,
) -> Result<WorkspacePackageGateRequest, String> {
    let args = args.collect::<Vec<_>>();
    let workspace_root = match args.as_slice() {
        [] => env::current_dir()
            .map_err(|error| format!("cannot resolve current directory: {error}"))?,
        [flag, path] if flag == "--workspace-root" => PathBuf::from(path),
        _ => {
            return Err(
                "usage: marlin-workspace-package-gate [--workspace-root <path>]".to_owned(),
            );
        }
    };
    WorkspacePackageGateRequest::new(workspace_root)
}

fn load_cargo_metadata(workspace_root: &Path) -> Result<CargoMetadata, String> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let output = Command::new(&cargo)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--locked",
            "--manifest-path",
        ])
        .arg(workspace_root.join("Cargo.toml"))
        .output()
        .map_err(|error| format!("cannot run {:?} metadata: {error}", cargo))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cannot decode cargo metadata: {error}"))
}
