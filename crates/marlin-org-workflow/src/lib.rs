//! Workflow facade for loading, planning, and committing `Org` workspace patches.

mod gerbil_intent;
mod patch_ops;
mod source_commit;

pub use gerbil_intent::GerbilWorkspacePatchIntentDryRunner;
pub use source_commit::{
    OrgWorkspaceSourceCommit, OrgWorkspaceSourceCommitReceipt, OrgWorkspaceSourceCommitter,
};
