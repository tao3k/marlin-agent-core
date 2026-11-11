// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Asynchronous read-only worktree observations.

use std::path::PathBuf;

use super::viewport::Transcript;
use marlin_git_utils::{
    GitDiff, GitDiffRequest, GitDiffScope, GitRepositoryPath, GitStatus, ProcessGitTooling,
};
use tokio::{runtime::Runtime, sync::oneshot, task::JoinHandle};

#[derive(Default)]
pub(crate) struct WorkspaceStatus {
    pending: Option<PendingStatus>,
    retiring: Option<JoinHandle<()>>,
    view: Option<WorkspaceView>,
}

pub(crate) struct WorkspaceView {
    pub(crate) transcript: Transcript,
    pub(crate) scroll_from_top: usize,
}

impl WorkspaceView {
    pub(crate) fn from_observation(observation: WorkspaceObservation) -> Self {
        Self {
            transcript: Transcript::new(vec![(
                "Workspace".into(),
                format_observation(&observation),
            )]),
            scroll_from_top: 0,
        }
    }
}

pub(crate) enum WorkspaceQuery {
    Status,
    Diff(GitDiffRequest),
}

pub(crate) fn parse_query(input: &str) -> Result<WorkspaceQuery, &'static str> {
    if input == "/status" {
        return Ok(WorkspaceQuery::Status);
    }
    let Some(arguments) = input.strip_prefix("/diff") else {
        return Err("Unknown workspace command.");
    };
    let arguments = arguments.trim();
    let (scope, selected) = if arguments == "staged" {
        (GitDiffScope::Index, "")
    } else if let Some(path) = arguments.strip_prefix("staged ") {
        (GitDiffScope::Index, path.trim())
    } else {
        (GitDiffScope::Worktree, arguments)
    };
    let request = if selected.is_empty() {
        GitDiffRequest::all(scope)
    } else {
        let path: String = serde_json::from_str(selected).map_err(
            |_| "Use /diff [staged] followed by a JSON-quoted repository-relative path.",
        )?;
        let path = GitRepositoryPath::new(path).map_err(|_| "Diff path must be a nonempty repository-relative path without parent components or NUL.")?;
        GitDiffRequest::for_path(scope, path)
    };
    Ok(WorkspaceQuery::Diff(request))
}

pub(crate) enum WorkspaceObservation {
    Status(GitStatus),
    Diff(GitDiff),
}

struct PendingStatus {
    receiver: oneshot::Receiver<Result<WorkspaceView, String>>,
    task: JoinHandle<()>,
}

impl WorkspaceStatus {
    pub(crate) fn start(
        &mut self,
        runtime: &Runtime,
        path: PathBuf,
        query: WorkspaceQuery,
    ) -> bool {
        if self.pending.is_some()
            || self
                .retiring
                .as_ref()
                .is_some_and(|task| !task.is_finished())
        {
            return false;
        }
        self.retiring = None;
        let (tx, rx) = oneshot::channel();
        let task = runtime.spawn(async move {
            let result = match query {
                WorkspaceQuery::Status => ProcessGitTooling::status(&path)
                    .await
                    .map(WorkspaceObservation::Status)
                    .map_err(|error| error.to_string()),
                WorkspaceQuery::Diff(request) => ProcessGitTooling::diff(&path, request)
                    .await
                    .map(WorkspaceObservation::Diff)
                    .map_err(|error| error.to_string()),
            };
            let result = result.map(WorkspaceView::from_observation);
            let _ = tx.send(result);
        });
        self.pending = Some(PendingStatus { receiver: rx, task });
        true
    }

    pub(crate) fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub(crate) fn show(&mut self, view: WorkspaceView) {
        self.view = Some(view);
    }

    pub(crate) fn view(&self) -> Option<&WorkspaceView> {
        self.view.as_ref()
    }
    pub(crate) fn view_mut(&mut self) -> Option<&mut WorkspaceView> {
        self.view.as_mut()
    }
    pub(crate) fn close_view(&mut self) -> bool {
        self.view.take().is_some()
    }

    pub(crate) fn cancel(&mut self) -> bool {
        if let Some(pending) = self.pending.take() {
            pending.task.abort();
            self.retiring = Some(pending.task);
            true
        } else {
            false
        }
    }

    pub(crate) fn poll(&mut self) -> Option<Result<WorkspaceView, String>> {
        let pending = self.pending.as_mut()?;
        let result = match pending.receiver.try_recv() {
            Ok(result) => result,
            Err(oneshot::error::TryRecvError::Empty) => return None,
            Err(oneshot::error::TryRecvError::Closed) => {
                Err("workspace status task ended unexpectedly".into())
            }
        };
        self.pending = None;
        Some(result)
    }
}

pub(crate) fn format_observation(observation: &WorkspaceObservation) -> String {
    match observation {
        WorkspaceObservation::Status(status) => format_status(status),
        WorkspaceObservation::Diff(diff) => {
            let label = match diff.request.scope {
                GitDiffScope::Worktree => "Unstaged diff",
                GitDiffScope::Index => "Staged diff",
            };
            let selection = diff
                .request
                .path
                .as_ref()
                .map(|path| {
                    format!(
                        " for \"{}\"",
                        path.as_str()
                            .chars()
                            .flat_map(char::escape_debug)
                            .collect::<String>()
                    )
                })
                .unwrap_or_default();
            if diff.as_bytes().is_empty() {
                if diff.request.path.is_some() {
                    format!(
                        "{label}{selection}: no tracked changes; the selected path may be absent or untracked."
                    )
                } else {
                    format!("{label}: no tracked changes.")
                }
            } else {
                format!("{label}{selection}:\n{}", diff.display_escaped())
            }
        }
    }
}

impl Drop for WorkspaceStatus {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub(crate) fn format_status(status: &GitStatus) -> String {
    if status.entries.is_empty() {
        return "Worktree clean (tracked and untracked files).".into();
    }
    let mut output = format!("{} status entries (index, worktree):", status.entries.len());
    for entry in &status.entries {
        output.push('\n');
        output.push(entry.index.code());
        output.push(entry.worktree.code());
        output.push(' ');
        if let Some(original) = &entry.original_path {
            output.push_str(&original.display_escaped());
            output.push_str(" -> ");
        }
        output.push_str(&entry.path.display_escaped());
    }
    output
}
