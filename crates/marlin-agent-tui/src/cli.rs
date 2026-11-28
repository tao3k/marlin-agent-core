// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Command-line selection and read-only session listing.

use super::config;
use marlin_agent_storage::SessionEventStore;
use marlin_agent_storage::{
    AgentId, ProjectId, SessionId, SessionPageRequest, StoragePageLimit, TursoAgentStorage,
    TursoAgentStorageConfig, TursoBatchTransactionMode, TursoOptimizationProfile,
};
use std::{
    ffi::OsString,
    io::{self, Write},
    path::PathBuf,
};

pub(crate) struct CliArgs {
    pub(crate) config: PathBuf,
    pub(crate) action: CliAction,
}

pub(crate) enum CliAction {
    Open {
        session: Option<SessionId>,
    },
    List {
        after: Option<SessionId>,
    },
    #[cfg(unix)]
    EnrollContext {
        remote: Option<marlin_agent_gitops::GitRemoteName>,
    },
}

pub(crate) fn parse_cli(args: impl Iterator<Item = OsString>) -> Result<CliArgs, String> {
    let args: Vec<_> = args.collect();
    let session_id = |value: &OsString| -> Result<SessionId, String> {
        let value = value.to_str().ok_or("session ID must be valid UTF-8")?;
        SessionId::new(value).map_err(|error| error.to_string())
    };
    let (config, action) = match args.as_slice() {
        [config] => (config, CliAction::Open { session: None }),
        [config, option, session] if option == "--session" => {
            (config, CliAction::Open { session: Some(session_id(session)?) })
        }
        [config, option] if option == "--list-sessions" => (config, CliAction::List { after: None }),
        [config, option, after, id] if option == "--list-sessions" && after == "--after" => (config, CliAction::List { after: Some(session_id(id)?) }),
        #[cfg(unix)]
        [config, option] if option == "--enroll-context" => (config, CliAction::EnrollContext { remote: None }),
        #[cfg(unix)]
        [config, option, remote, name] if option == "--enroll-context" && remote == "--remote" => {
            let name = name.to_str().ok_or("Git remote name must be valid UTF-8")?;
            let name = marlin_agent_gitops::GitRemoteName::new(name).map_err(|error| error.to_string())?;
            (config, CliAction::EnrollContext { remote: Some(name) })
        }
        _ => return Err("Usage: marlin-tui <config.toml> [--session <session-id> | --list-sessions [--after <session-id>] | --enroll-context [--remote <name>]]".into()),
    };
    Ok(CliArgs {
        config: config.into(),
        action,
    })
}

#[cfg(unix)]
pub(crate) fn enroll_context(
    path: &std::path::Path,
    remote: Option<marlin_agent_gitops::GitRemoteName>,
) -> Result<(), Box<dyn std::error::Error>> {
    use marlin_agent_gitops::{EnrolledRepository, GitOperations};

    let approval = config::Config::load(path)?
        .git_context
        .ok_or("--enroll-context requires a [git_context] repository")?;
    let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let owner = runtime.block_on(async {
        let context = GitOperations::context(&approval.repository).await?;
        Ok::<_, Box<dyn std::error::Error>>(
            EnrolledRepository::enroll(std::path::Path::new(&home), &context, remote.as_ref())
                .await?,
        )
    })?;
    writeln!(io::stdout().lock(), "{}", owner.root().display())?;
    Ok(())
}

pub(crate) fn list_sessions(
    path: &std::path::Path,
    after: Option<SessionId>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::load(path)?;
    let session = config
        .session
        .ok_or("--list-sessions requires a [session] storage configuration")?;
    if !session.path.exists() {
        return Err("session database does not exist".into());
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let page = runtime.block_on(async {
        let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
            path: session.path,
            optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
            batch_transaction_mode: TursoBatchTransactionMode::Immediate,
        })
        .await?;
        let mut request = SessionPageRequest::new(
            ProjectId::new(session.project)?,
            AgentId::new(session.agent)?,
            StoragePageLimit::new(50)?,
        );
        if let Some(cursor) = after {
            request = request.after(cursor);
        }
        storage.list_sessions_page(request).await
    })?;
    let mut output = io::stdout().lock();
    serde_json::to_writer(&mut output, &page)?;
    writeln!(output)?;
    Ok(())
}
