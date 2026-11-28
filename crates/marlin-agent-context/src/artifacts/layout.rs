// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Declarative workspace storage and session layout projected from Scheme `artifacts`.

use std::{
    io,
    path::{Component, Path, PathBuf},
};

use gerbil_scheme::{CollectionKind, GerbilRuntime, IdentifierKind, RootedSchemeObject};

use super::workspace::{ArtifactResult, WorkspaceArtifacts, ensure_directory};

/// Host-admitted workspace artifact storage layout projected from Scheme's
/// `DefaultArtifactCatalog.workspace-layout` over the zero-JSON C-ABI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceArtifactLayout {
    pub root: PathBuf,
    pub session_db_path: PathBuf,
    pub context_db_path: PathBuf,
    pub search_dir: PathBuf,
    pub process_ledger_path: PathBuf,
    pub project: String,
    pub session: String,
    pub agent: String,
    pub development: bool,
}

impl WorkspaceArtifacts {
    /// Project the declarative storage/session layout from Scheme `artifacts`
    /// (`#('workspace-layout)`) and ensure the owning workspace directories exist.
    pub fn project_layout<'vm>(
        &self,
        runtime: &'vm GerbilRuntime,
        exchange: impl FnOnce(&RootedSchemeObject<'vm>) -> ArtifactResult<RootedSchemeObject<'vm>>,
    ) -> ArtifactResult<WorkspaceArtifactLayout> {
        self.check_enrollment()?;
        let action = runtime.intern_identifier(IdentifierKind::Symbol, "workspace-layout")?;
        let request = runtime
            .collection_from_objects(CollectionKind::Vector, &[&action])?
            .into_object();
        let response = exchange(&request)?;
        let decoded = RawWorkspaceLayout::decode(response)?;
        self.ensure_layout_directories(&decoded.search_dir)?;

        let root = self.root().to_path_buf();
        let session_db_path = root.join(&decoded.session_db);
        let context_db_path = root.join(&decoded.context_db);
        let search_dir = root.join(&decoded.search_dir);
        let mut ledger_os = session_db_path.as_os_str().to_os_string();
        ledger_os.push(&decoded.process_ledger_suffix);
        let process_ledger_path = PathBuf::from(ledger_os);

        Ok(WorkspaceArtifactLayout {
            root,
            session_db_path,
            context_db_path,
            search_dir,
            process_ledger_path,
            project: decoded.project,
            session: decoded.session,
            agent: decoded.agent,
            development: self.is_development_mode(),
        })
    }

    fn ensure_layout_directories(&self, search_dir: &str) -> io::Result<()> {
        let _lock = self.open_lock()?;
        ensure_directory(&self.root().join("artifacts"))?;
        ensure_directory(&self.root().join(search_dir))?;
        Ok(())
    }
}

struct RawWorkspaceLayout {
    session_db: String,
    context_db: String,
    search_dir: String,
    process_ledger_suffix: String,
    project: String,
    session: String,
    agent: String,
}

impl RawWorkspaceLayout {
    fn decode(response: RootedSchemeObject<'_>) -> ArtifactResult<Self> {
        let vec = response.into_collection(CollectionKind::Vector)?;
        if vec.len() != 7 {
            return Err("invalid workspace layout projection".into());
        }
        let read_str = |index: usize| -> ArtifactResult<String> {
            let value = vec
                .element(index)?
                .into_string()?
                .to_string()
                .into_result()?;
            if value.trim().is_empty() {
                return Err(format!("empty workspace layout slot {index}").into());
            }
            Ok(value)
        };
        let session_db = admit_relative_segment(read_str(0)?, "session-db")?;
        let context_db = admit_relative_segment(read_str(1)?, "context-db")?;
        let search_dir = admit_relative_segment(read_str(2)?, "search-dir")?;
        let process_ledger_suffix = read_str(3)?;
        if process_ledger_suffix.contains('/') || process_ledger_suffix.contains('\\') {
            return Err("invalid process-ledger-suffix in workspace layout".into());
        }
        let project = read_str(4)?;
        let session = read_str(5)?;
        let agent = read_str(6)?;
        Ok(Self {
            session_db,
            context_db,
            search_dir,
            process_ledger_suffix,
            project,
            session,
            agent,
        })
    }
}

fn admit_relative_segment(raw: String, field: &str) -> ArtifactResult<String> {
    let path = Path::new(&raw);
    let mut parts = path.components();
    match (parts.next(), parts.next()) {
        (Some(Component::Normal(_)), None) => Ok(raw),
        _ => {
            Err(format!("workspace layout `{field}` must be a single relative path segment").into())
        }
    }
}
