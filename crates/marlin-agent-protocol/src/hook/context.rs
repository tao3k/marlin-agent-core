//! Typed context facts for hook policy selection.

use serde::{Deserialize, Serialize};

macro_rules! hook_context_newtype {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }
    };
}

hook_context_newtype!(
    HookSessionId,
    "Stable session identifier observed by hook policy evaluation."
);
hook_context_newtype!(
    HookAgentLineageNode,
    "Agent lineage node observed by hook policy evaluation."
);
hook_context_newtype!(
    HookWorkspaceStateFact,
    "Workspace state fact observed by hook policy evaluation."
);
hook_context_newtype!(
    HookOrgMemoryHit,
    "Org memory hit observed by hook policy evaluation."
);
hook_context_newtype!(
    HookAgentClass,
    "Custom/customer agent class observed by hook policy evaluation."
);

/// Typed context facts available to hook policy selection and extension finalizers.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HookDecisionContext {
    pub session_id: Option<HookSessionId>,
    pub agent_lineage: Vec<HookAgentLineageNode>,
    pub workspace_state: Vec<HookWorkspaceStateFact>,
    pub org_memory_hits: Vec<HookOrgMemoryHit>,
    pub agent_class: Option<HookAgentClass>,
}

impl HookDecisionContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_session_id(mut self, session_id: impl Into<HookSessionId>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_agent_lineage_node(mut self, node: impl Into<HookAgentLineageNode>) -> Self {
        self.agent_lineage.push(node.into());
        self
    }

    pub fn with_workspace_state(mut self, fact: impl Into<HookWorkspaceStateFact>) -> Self {
        self.workspace_state.push(fact.into());
        self
    }

    pub fn with_org_memory_hit(mut self, hit: impl Into<HookOrgMemoryHit>) -> Self {
        self.org_memory_hits.push(hit.into());
        self
    }

    pub fn with_agent_class(mut self, agent_class: impl Into<HookAgentClass>) -> Self {
        self.agent_class = Some(agent_class.into());
        self
    }
}
