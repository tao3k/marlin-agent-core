//! Session ancestry and runtime context contracts.

use crate::{ContextNamespace, ContextVisibility, SessionId, SessionIsolationPolicy, SessionKind};
use serde::{Deserialize, Serialize};

/// Session ancestry propagated through runtime child contexts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionIdentity {
    session_id: SessionId,
    parent_session_id: Option<SessionId>,
    root_session_id: SessionId,
    kind: SessionKind,
}

impl SessionIdentity {
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub fn parent_session_id(&self) -> Option<&SessionId> {
        self.parent_session_id.as_ref()
    }

    pub fn root_session_id(&self) -> &SessionId {
        &self.root_session_id
    }

    pub fn kind(&self) -> &SessionKind {
        &self.kind
    }
}

/// Runtime session context carried by providers, tools, hooks, and sub-agents.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentSessionContext {
    identity: SessionIdentity,
    visibility: ContextVisibility,
    isolation: SessionIsolationPolicy,
}

impl AgentSessionContext {
    pub fn root(session_id: impl Into<SessionId>, visibility: ContextVisibility) -> Self {
        Self::root_with_policy(session_id, visibility, SessionIsolationPolicy::strict())
    }

    pub fn root_with_policy(
        session_id: impl Into<SessionId>,
        visibility: ContextVisibility,
        isolation: SessionIsolationPolicy,
    ) -> Self {
        let session_id = session_id.into();
        Self {
            identity: SessionIdentity {
                session_id: session_id.clone(),
                parent_session_id: None,
                root_session_id: session_id,
                kind: SessionKind::Root,
            },
            visibility,
            isolation,
        }
    }

    pub fn runtime_root() -> Self {
        Self::root("runtime.root", ContextVisibility::default_runtime())
    }

    pub fn child_session(
        &self,
        kind: SessionKind,
        child_session_id: impl Into<SessionId>,
        requested_visibility: ContextVisibility,
    ) -> (Self, SessionIsolationReceipt) {
        let child_session_id = child_session_id.into();
        let (granted_visibility, denied_namespaces, history_limit_applied) = self
            .isolation
            .grant_visibility(&self.visibility, &requested_visibility);
        let child = Self {
            identity: SessionIdentity {
                session_id: child_session_id.clone(),
                parent_session_id: Some(self.identity.session_id.clone()),
                root_session_id: self.identity.root_session_id.clone(),
                kind,
            },
            visibility: granted_visibility.clone(),
            isolation: self.isolation.clone(),
        };
        let receipt = SessionIsolationReceipt {
            parent_session_id: self.identity.session_id.clone(),
            child_session_id,
            requested_visibility,
            granted_visibility,
            denied_namespaces,
            history_limit_applied,
        };

        (child, receipt)
    }

    pub fn identity(&self) -> &SessionIdentity {
        &self.identity
    }

    pub fn session_id(&self) -> &SessionId {
        self.identity.session_id()
    }

    pub fn parent_session_id(&self) -> Option<&SessionId> {
        self.identity.parent_session_id()
    }

    pub fn root_session_id(&self) -> &SessionId {
        self.identity.root_session_id()
    }

    pub fn kind(&self) -> &SessionKind {
        self.identity.kind()
    }

    pub fn visibility(&self) -> &ContextVisibility {
        &self.visibility
    }

    pub fn isolation(&self) -> &SessionIsolationPolicy {
        &self.isolation
    }
}

impl Default for AgentSessionContext {
    fn default() -> Self {
        Self::runtime_root()
    }
}

/// Audit record emitted when a child session visibility request is narrowed.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionIsolationReceipt {
    parent_session_id: SessionId,
    child_session_id: SessionId,
    requested_visibility: ContextVisibility,
    granted_visibility: ContextVisibility,
    denied_namespaces: Vec<ContextNamespace>,
    history_limit_applied: bool,
}

impl SessionIsolationReceipt {
    pub fn parent_session_id(&self) -> &SessionId {
        &self.parent_session_id
    }

    pub fn child_session_id(&self) -> &SessionId {
        &self.child_session_id
    }

    pub fn requested_visibility(&self) -> &ContextVisibility {
        &self.requested_visibility
    }

    pub fn granted_visibility(&self) -> &ContextVisibility {
        &self.granted_visibility
    }

    pub fn denied_namespaces(&self) -> &[ContextNamespace] {
        self.denied_namespaces.as_slice()
    }

    pub fn history_limit_applied(&self) -> bool {
        self.history_limit_applied
    }
}
