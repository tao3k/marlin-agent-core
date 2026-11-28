// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Staged restoration across the host worker and the owning Scheme thread.

use std::{fs::File, io, path::PathBuf};

use gerbil_scheme::{GerbilRuntime, RootedSchemeObject};

use super::workspace::{
    ArtifactProjection, ArtifactResult, ProjectionContents, WorkspaceArtifacts, regular_or_missing,
    restore_request,
};

impl WorkspaceArtifacts {
    /// Lock an existing journal without creating storage in a new checkout.
    pub fn prepare_restore(self) -> ArtifactResult<Option<ArtifactRestore>> {
        let path = self.root().join("context.db");
        if !regular_or_missing(&path)? {
            return Ok(None);
        }
        let lock = self.open_lock()?;
        lock.try_lock().map_err(io::Error::from)?;
        if !regular_or_missing(&self.checked_journal()?)? {
            return Ok(None);
        }
        Ok(Some(ArtifactRestore {
            workspace: self,
            journal: path,
            _lock: lock,
        }))
    }
}

/// An existing owner-locked journal ready to read on the Scheme thread.
pub struct ArtifactRestore {
    workspace: WorkspaceArtifacts,
    journal: PathBuf,
    _lock: File,
}

impl ArtifactRestore {
    pub fn project<'vm>(
        self,
        runtime: &'vm GerbilRuntime,
        exchange: impl FnOnce(&RootedSchemeObject<'vm>) -> ArtifactResult<RootedSchemeObject<'vm>>,
    ) -> ArtifactResult<ArtifactRestoreDraft> {
        self.workspace.check_enrollment()?;
        let request = restore_request(runtime, &self.journal)?;
        let contents = ProjectionContents::decode(exchange(&request)?)?;
        Ok(ArtifactRestoreDraft {
            restore: self,
            contents,
        })
    }
}

/// An Org projection detached from Scheme roots for physical file verification.
pub struct ArtifactRestoreDraft {
    restore: ArtifactRestore,
    contents: ProjectionContents,
}

impl ArtifactRestoreDraft {
    pub fn verify(self) -> ArtifactResult<VerifiedArtifactRestore> {
        let files = self.contents.verify_files(self.restore.workspace.root())?;
        Ok(VerifiedArtifactRestore {
            restore: self.restore,
            org: self.contents.org,
            files,
        })
    }
}

/// Ready for publication after the current checkout's admission is checked.
pub struct VerifiedArtifactRestore {
    restore: ArtifactRestore,
    org: String,
    files: Vec<PathBuf>,
}

impl VerifiedArtifactRestore {
    pub fn publish(self) -> ArtifactResult<ArtifactProjection> {
        self.restore.workspace.publish_org(&self.org, self.files)
    }
}
