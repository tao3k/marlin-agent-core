// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Terminal admission and status for the Scheme-selected project environment.

use std::{env, path::Path};

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, SchemeScalar,
};
use marlin_agent_runtime::{EnvrcIntent, EnvrcPlan, EnvrcStatus, ProjectEnvrc, TokioAgentRuntime};

use crate::command::EnvrcCommand;

use super::{App, MessageEvent, MessageSeverity, MessageSource};

pub(super) async fn project(directory: &Path) -> Option<ProjectEnvrc> {
    let root = marlin_agent_gitops::GitOperations::resolve_repository_root(directory)
        .await
        .ok()?;
    // Give direnv the host paths it needs to run and find its approval store.
    // Only values exported by the selected .envrc flow into the clean tool env.
    let base = [
        "HOME",
        "PATH",
        "SHELL",
        "TMPDIR",
        "USER",
        "LOGNAME",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_CACHE_HOME",
        "XDG_RUNTIME_DIR",
    ]
    .into_iter()
    .filter_map(|name| env::var(name).ok().map(|value| (name.to_owned(), value)))
    .collect();
    Some(ProjectEnvrc::new(root, base))
}

pub(super) fn apply(
    app: &mut App,
    tokio: &tokio::runtime::Runtime,
    scheme: &GerbilRuntime,
    command: EnvrcCommand,
) {
    let Some(binding) = app.runtime_owner.project_envrc() else {
        if let Some(status) = app.runtime_owner.project_envrc_status() {
            publish_status(app, status);
        } else {
            publish(app, MessageSeverity::Info, "No active Git worktree.");
        }
        return;
    };
    match command {
        EnvrcCommand::Status => {
            let status = binding.status();
            publish_status(app, status);
        }
        EnvrcCommand::Cancel => {
            let detail = if binding.cancel_pending() {
                "Environment cancellation requested."
            } else {
                "There is no environment load to cancel."
            };
            publish(app, MessageSeverity::Info, detail);
        }
        EnvrcCommand::Apply(intent) => {
            let _entered = tokio.enter();
            submit(&app.runtime_owner, scheme, &binding, intent);
        }
    }
}

pub(super) fn submit(
    runtime: &TokioAgentRuntime,
    scheme: &GerbilRuntime,
    binding: &ProjectEnvrc,
    intent: EnvrcIntent,
) {
    // Runtime rechecks the file before executing to invalidate stale state.
    let exists = binding.has_file().unwrap_or(false);
    match scheme_plan(scheme, intent, exists) {
        Ok(plan) => binding.submit(runtime, intent, plan),
        Err(_) => binding.reject(intent),
    }
}

pub(super) fn report_changes(app: &mut App, observed: &mut Option<EnvrcStatus>) -> bool {
    let status = app.runtime_owner.project_envrc_status();
    if *observed == status {
        return false;
    }
    *observed = status;
    if let Some(status) = status
        && !matches!(status, EnvrcStatus::Absent | EnvrcStatus::Loading)
    {
        publish_status(app, status);
    }
    true
}

fn publish_status(app: &mut App, status: EnvrcStatus) {
    let (severity, detail) = match status {
        EnvrcStatus::Blocked => (
            MessageSeverity::Warning,
            "Use M-x envrc-allow to approve this .envrc.",
        ),
        EnvrcStatus::TimedOut | EnvrcStatus::Failed => (
            MessageSeverity::Warning,
            "Use M-x envrc-reload to try again.",
        ),
        _ => (MessageSeverity::Info, ""),
    };
    let summary = format!("Worktree ENVRC {}", status.label());
    app.publish_message(MessageEvent {
        source: MessageSource::Runtime,
        severity,
        summary: &summary,
        detail,
    });
}

fn publish(app: &mut App, severity: MessageSeverity, detail: &str) {
    app.publish_message(MessageEvent {
        source: MessageSource::Runtime,
        severity,
        summary: "Worktree ENVRC",
        detail,
    });
}

fn scheme_plan(
    runtime: &GerbilRuntime,
    intent: EnvrcIntent,
    exists: bool,
) -> Result<EnvrcPlan, Box<dyn std::error::Error>> {
    let action = runtime.intern_identifier(IdentifierKind::Symbol, intent.symbol())?;
    let present = runtime.scalar_object(SchemeScalar::Boolean(exists))?;
    let request = runtime
        .collection_from_objects(CollectionKind::Vector, &[&action, &present])?
        .into_object();
    #[cfg(feature = "linked-ui")]
    let value = crate::linked_ui::envrc_plan(runtime, &request)?;
    #[cfg(not(feature = "linked-ui"))]
    let value = {
        let directory = env::current_dir()?;
        let directory = directory
            .to_str()
            .ok_or("ENVRC package directory is not UTF-8")?;
        let module =
            runtime.intern_identifier(IdentifierKind::Symbol, ":marlin-agent-core/envrc/bridge")?;
        let export = runtime.intern_identifier(IdentifierKind::Symbol, "plan-project-envrc")?;
        let procedure = runtime.resolve_package_export(directory, &module, &export)?;
        runtime.call_rooted(&procedure, &request)?
    };
    let mut symbols = IdentifierBuffer::default();
    value.as_ref().copy_identifier_names(&mut symbols)?;
    match (symbols.len(), symbols.get(0)) {
        (1, Some((IdentifierKind::Symbol, plan))) => {
            EnvrcPlan::decode(intent, exists, plan).ok_or_else(|| "invalid ENVRC plan".into())
        }
        _ => Err("ENVRC policy did not return a plan symbol".into()),
    }
}
