// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::path::PathBuf;

use marlin_workspace_protocol::{
    WorkingCopyCommandProjectionError, WorkingCopyGitTopLevel, WorkingCopyRepositoryDiscoveryPath,
};

mod git_worktree;
mod pull_request;
mod receipts;
mod worktrunk;

fn test_git_toplevel_resolver(
    path: &WorkingCopyRepositoryDiscoveryPath,
) -> Result<WorkingCopyGitTopLevel, WorkingCopyCommandProjectionError> {
    let resolved = if path.as_path() == PathBuf::from("/repo/subdir").as_path() {
        PathBuf::from("/repo")
    } else {
        path.as_path().to_path_buf()
    };
    Ok(WorkingCopyGitTopLevel::from_resolved_path(resolved))
}
