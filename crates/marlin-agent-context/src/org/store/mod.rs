//! Org source persistence and discovery boundaries.

mod discovery;
mod file;
mod memory;
mod release_status;
mod source;

pub use discovery::{
    OrgProjectRoot, OrgProjectRootCandidate, OrgProjectRootKind, discover_project_roots,
};
pub use file::FileSystemOrgSourceStore;
pub use memory::MemoryOrgSourceStore;
pub use release_status::FileSystemReleaseStatusStore;
pub use source::{
    OrgAtomicSourceStore, OrgAtomicWriteOutcome, OrgSourceCleanState, OrgSourceDocumentId,
    OrgSourceStore, OrgSourceStoreError, OrgSourceStoreResult,
};
