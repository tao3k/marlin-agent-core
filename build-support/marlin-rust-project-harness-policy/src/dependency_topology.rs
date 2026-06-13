//! External dependency topology receipt interface for build-time agent repair guidance.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

const RUST_HARNESS_PACKAGE: &str = "rust-lang-project-harness";
const RECEIPT_ENV: &str = "MARLIN_DEPENDENCY_TOPOLOGY_RECEIPT";
const RECEIPT_STRICT_ENV: &str = "MARLIN_DEPENDENCY_TOPOLOGY_RECEIPT_STRICT";
const RECEIPT_SCHEMA_ID: &str = "agent.semantic-protocols.dependency-topology-receipt";

/// Result status for consuming an externally produced dependency topology receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExternalDependencyTopologyReceiptStatus {
    Verified,
    NotConfigured,
    MissingReceipt,
    ReceiptReadError(String),
    InvalidReceipt(String),
    PackageMismatch { expected: String, actual: String },
    ProducerReportedIssue(String),
}

/// Receipt describing Marlin's consumption of external dependency topology evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalDependencyTopologyReceipt {
    package_name: String,
    receipt_path: Option<PathBuf>,
    producer: Option<String>,
    evidence_sources: Vec<String>,
    status: ExternalDependencyTopologyReceiptStatus,
    agent_next_action: Option<String>,
    verification_command: Option<String>,
}

impl ExternalDependencyTopologyReceipt {
    fn new(
        package_name: impl Into<String>,
        receipt_path: Option<PathBuf>,
        producer: Option<String>,
        evidence_sources: Vec<String>,
        status: ExternalDependencyTopologyReceiptStatus,
        agent_next_action: Option<String>,
        verification_command: Option<String>,
    ) -> Self {
        Self {
            package_name: package_name.into(),
            receipt_path,
            producer,
            evidence_sources,
            status,
            agent_next_action,
            verification_command,
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(
            self.status,
            ExternalDependencyTopologyReceiptStatus::Verified
                | ExternalDependencyTopologyReceiptStatus::NotConfigured
        )
    }

    pub fn blocks_build(&self, strict: bool) -> bool {
        strict && !self.is_success()
    }

    pub fn package_name(&self) -> &str {
        self.package_name.as_str()
    }

    pub fn receipt_path(&self) -> Option<&Path> {
        self.receipt_path.as_deref()
    }

    pub fn producer(&self) -> Option<&str> {
        self.producer.as_deref()
    }

    pub fn evidence_sources(&self) -> &[String] {
        self.evidence_sources.as_slice()
    }

    pub fn status(&self) -> &ExternalDependencyTopologyReceiptStatus {
        &self.status
    }

    pub fn agent_next_action(&self) -> Option<&str> {
        self.agent_next_action.as_deref()
    }

    pub fn verification_command(&self) -> Option<&str> {
        self.verification_command.as_deref()
    }

    pub fn agent_message(&self) -> String {
        let receipt = self
            .receipt_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "-".to_owned());
        let producer = self.producer.as_deref().unwrap_or("-");
        let evidence = if self.evidence_sources.is_empty() {
            "-".to_owned()
        } else {
            self.evidence_sources.join(", ")
        };
        let default_action = if matches!(
            self.status,
            ExternalDependencyTopologyReceiptStatus::NotConfigured
        ) {
            "no generated topology data is required for hermetic builds; optionally set MARLIN_DEPENDENCY_TOPOLOGY_RECEIPT for ASP graph turbo or rust harness topology evidence"
        } else {
            "regenerate the external dependency topology receipt with ASP graph turbo or rust harness"
        };
        let action = self.agent_next_action.as_deref().unwrap_or(default_action);
        let command = self.verification_command.as_deref().unwrap_or("-");
        format!(
            "[MARLIN-DEPTOPO-IFACE-001] External dependency topology receipt check failed\npackage={}\nreceipt={receipt}\nproducer={producer}\nstatus={:?}\nevidence={evidence}\naction={action}\nverify={command}",
            self.package_name, self.status
        )
    }
}

/// Observe the optional Rust harness topology receipt from the build environment.
pub fn observe_rust_harness_dependency_topology_receipt_from_env(
    project_root: &Path,
) -> ExternalDependencyTopologyReceipt {
    println!("cargo:rerun-if-env-changed={RECEIPT_ENV}");
    println!("cargo:rerun-if-env-changed={RECEIPT_STRICT_ENV}");
    let receipt_path = env::var_os(RECEIPT_ENV).map(PathBuf::from);
    consume_external_dependency_topology_receipt(
        project_root,
        RUST_HARNESS_PACKAGE,
        receipt_path.as_deref(),
    )
}

/// Observe the configured Rust harness topology receipt and gate it only in strict mode.
pub fn assert_rust_harness_dependency_topology_receipt_from_env_if_strict(
    project_root: &Path,
) -> ExternalDependencyTopologyReceipt {
    let strict = dependency_topology_receipt_strict_from_env();
    let receipt = observe_rust_harness_dependency_topology_receipt_from_env(project_root);
    assert!(!receipt.blocks_build(strict), "{}", receipt.agent_message());
    receipt
}

/// Consume one externally produced dependency topology receipt.
pub fn consume_external_dependency_topology_receipt(
    project_root: &Path,
    package_name: &str,
    receipt_path: Option<&Path>,
) -> ExternalDependencyTopologyReceipt {
    let Some(receipt_path) = receipt_path else {
        return ExternalDependencyTopologyReceipt::new(
            package_name,
            None,
            None,
            Vec::new(),
            ExternalDependencyTopologyReceiptStatus::NotConfigured,
            Some(format!(
                "no external topology receipt is required for hermetic builds; optionally set {RECEIPT_ENV} for {package_name}"
            )),
            Some(format!(
                "asp rust search pipe 'dependency topology {package_name}' --workspace {} --view seeds",
                project_root.display()
            )),
        );
    };

    let receipt_path = receipt_path.to_path_buf();
    if !receipt_path.is_file() {
        return ExternalDependencyTopologyReceipt::new(
            package_name,
            Some(receipt_path),
            None,
            Vec::new(),
            ExternalDependencyTopologyReceiptStatus::MissingReceipt,
            Some(format!(
                "regenerate the external topology receipt for {package_name} and update {RECEIPT_ENV}"
            )),
            None,
        );
    }

    let receipt = match fs::read_to_string(&receipt_path) {
        Ok(receipt) => receipt,
        Err(error) => {
            return ExternalDependencyTopologyReceipt::new(
                package_name,
                Some(receipt_path),
                None,
                Vec::new(),
                ExternalDependencyTopologyReceiptStatus::ReceiptReadError(error.to_string()),
                Some("make the external topology receipt readable by the build script".to_owned()),
                None,
            );
        }
    };

    let document = match serde_json::from_str::<ExternalDependencyTopologyReceiptDocument>(&receipt)
    {
        Ok(document) => document,
        Err(error) => {
            return ExternalDependencyTopologyReceipt::new(
                package_name,
                Some(receipt_path),
                None,
                Vec::new(),
                ExternalDependencyTopologyReceiptStatus::InvalidReceipt(error.to_string()),
                Some(
                    "emit the dependency topology receipt using the rust harness schema".to_owned(),
                ),
                None,
            );
        }
    };

    document.into_receipt(project_root, package_name, receipt_path)
}

fn dependency_topology_receipt_strict_from_env() -> bool {
    env::var_os(RECEIPT_STRICT_ENV)
        .and_then(|value| value.into_string().ok())
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct ExternalDependencyTopologyReceiptDocument {
    schema_id: String,
    #[allow(dead_code)]
    schema_version: String,
    package_name: String,
    #[serde(default)]
    producer: Option<String>,
    status: String,
    #[serde(default, alias = "resolved_sources")]
    evidence_sources: Vec<String>,
    #[serde(default)]
    agent_next_action: Option<String>,
    #[serde(default)]
    verification_command: Option<String>,
}

impl ExternalDependencyTopologyReceiptDocument {
    fn into_receipt(
        self,
        project_root: &Path,
        expected_package_name: &str,
        receipt_path: PathBuf,
    ) -> ExternalDependencyTopologyReceipt {
        if self.schema_id != RECEIPT_SCHEMA_ID {
            return ExternalDependencyTopologyReceipt::new(
                expected_package_name,
                Some(receipt_path),
                self.producer,
                self.evidence_sources,
                ExternalDependencyTopologyReceiptStatus::InvalidReceipt(format!(
                    "expected schema_id {RECEIPT_SCHEMA_ID}, got {}",
                    self.schema_id
                )),
                Some(
                    "regenerate the receipt with the current rust harness topology schema"
                        .to_owned(),
                ),
                self.verification_command,
            );
        }

        if self.package_name != expected_package_name {
            return ExternalDependencyTopologyReceipt::new(
                expected_package_name,
                Some(receipt_path),
                self.producer,
                self.evidence_sources,
                ExternalDependencyTopologyReceiptStatus::PackageMismatch {
                    expected: expected_package_name.to_owned(),
                    actual: self.package_name,
                },
                Some(format!(
                    "generate the external topology receipt for {expected_package_name}"
                )),
                self.verification_command,
            );
        }

        let normalized_status = self.status.to_ascii_lowercase();
        let status = match normalized_status.as_str() {
            "passed" | "verified" | "ok" => ExternalDependencyTopologyReceiptStatus::Verified,
            other => {
                ExternalDependencyTopologyReceiptStatus::ProducerReportedIssue(other.to_owned())
            }
        };
        let agent_next_action = self.agent_next_action.or_else(|| {
            (!matches!(status, ExternalDependencyTopologyReceiptStatus::Verified)).then(|| {
                format!(
                    "regenerate the dependency topology receipt for {expected_package_name} in {}",
                    project_root.display()
                )
            })
        });

        ExternalDependencyTopologyReceipt::new(
            expected_package_name,
            Some(receipt_path),
            self.producer,
            self.evidence_sources,
            status,
            agent_next_action,
            self.verification_command,
        )
    }
}
