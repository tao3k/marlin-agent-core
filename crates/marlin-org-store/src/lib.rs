//! Source persistence protocol for planned `Org` text edits.

mod commit;
mod discovery;
mod file;
mod memory;
mod release_status;
mod store;

pub use commit::{
    OrgSourceCommit, OrgSourceCommitReceipt, OrgSourceCommitter, OrgSourceConflict,
    OrgSourceDiagnostic, OrgSourceDiagnosticKind, OrgSourceDocumentHash,
    OrgSourceMultiDocumentPolicy, OrgSourceWriteMode, OrgSourceWritePolicy,
};
pub use discovery::{
    OrgProjectRoot, OrgProjectRootCandidate, OrgProjectRootKind, discover_project_roots,
};
pub use file::FileSystemOrgSourceStore;
pub use memory::MemoryOrgSourceStore;
pub use release_status::FileSystemReleaseStatusStore;
pub use store::{OrgSourceStore, OrgSourceStoreError, OrgSourceStoreResult};
