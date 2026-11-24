// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Git-derived workspace and durable artifact I/O.

mod restore;
mod workspace;

pub use restore::{ArtifactRestore, ArtifactRestoreDraft, VerifiedArtifactRestore};
#[cfg(feature = "artifacts")]
pub use workspace::ArtifactKind;
pub use workspace::{ArtifactProjection, WorkspaceArtifacts};
