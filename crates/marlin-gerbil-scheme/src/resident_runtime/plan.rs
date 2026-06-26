//! Plan construction for resident `Gerbil Scheme` runtime sessions.

use std::{io, path::PathBuf};

use crate::{GerbilCommandProfile, default_gerbil_gxi_program, write_gerbil_runtime_assets};

use super::{
    GerbilResidentRuntimeHandle, GerbilResidentRuntimePlan, GerbilResidentRuntimeSessionId,
    GerbilResidentRuntimeSessionMode,
};

impl GerbilResidentRuntimePlan {
    pub fn disabled(loadpath_root: impl Into<PathBuf>) -> Self {
        let loadpath_root = loadpath_root.into();
        Self {
            session_mode: GerbilResidentRuntimeSessionMode::Disabled,
            session_id: None,
            command_profile: default_command_profile(&loadpath_root),
            loadpath_root,
        }
    }

    pub fn shared_context(
        loadpath_root: impl Into<PathBuf>,
        session_id: impl Into<GerbilResidentRuntimeSessionId>,
    ) -> Self {
        Self::new(
            GerbilResidentRuntimeSessionMode::SharedContext,
            loadpath_root,
            session_id,
        )
    }

    pub fn forked_context(
        loadpath_root: impl Into<PathBuf>,
        session_id: impl Into<GerbilResidentRuntimeSessionId>,
    ) -> Self {
        Self::new(
            GerbilResidentRuntimeSessionMode::ForkedContext,
            loadpath_root,
            session_id,
        )
    }

    pub fn isolated_session(
        loadpath_root: impl Into<PathBuf>,
        session_id: impl Into<GerbilResidentRuntimeSessionId>,
    ) -> Self {
        Self::new(
            GerbilResidentRuntimeSessionMode::IsolatedSession,
            loadpath_root,
            session_id,
        )
    }

    pub fn with_command_profile(mut self, command_profile: GerbilCommandProfile) -> Self {
        self.command_profile = command_profile;
        self
    }

    /// Writes runtime assets and returns a handle ready for resident process ownership.
    pub fn prepare(self) -> io::Result<GerbilResidentRuntimeHandle> {
        let written_assets = write_gerbil_runtime_assets(&self.loadpath_root)?;
        Ok(GerbilResidentRuntimeHandle {
            plan: self,
            written_assets,
        })
    }

    pub fn requires_process_reuse(&self) -> bool {
        self.session_mode != GerbilResidentRuntimeSessionMode::Disabled
    }

    pub fn isolates_state(&self) -> bool {
        self.session_mode == GerbilResidentRuntimeSessionMode::IsolatedSession
    }

    fn new(
        session_mode: GerbilResidentRuntimeSessionMode,
        loadpath_root: impl Into<PathBuf>,
        session_id: impl Into<GerbilResidentRuntimeSessionId>,
    ) -> Self {
        let loadpath_root = loadpath_root.into();
        Self {
            session_mode,
            session_id: Some(session_id.into()),
            command_profile: default_command_profile(&loadpath_root),
            loadpath_root,
        }
    }
}

fn default_command_profile(loadpath_root: impl Into<PathBuf>) -> GerbilCommandProfile {
    GerbilCommandProfile::marlin_runtime_module(
        default_gerbil_gxi_program().to_string_lossy().into_owned(),
        loadpath_root,
    )
}
