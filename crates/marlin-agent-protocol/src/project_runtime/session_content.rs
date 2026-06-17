//! Project-scoped session, content, and memory-trigger facts.

use serde::{Deserialize, Serialize};

use super::ids::{
    GraphQueryScoreBasisPoints, ProjectRuntimeContentId, ProjectRuntimeContextPackId,
    ProjectRuntimeMemoryCitationId, ProjectRuntimeMemoryId, ProjectRuntimeProjectId,
    ProjectRuntimeReceiptId, ProjectRuntimeRootSessionId, ProjectRuntimeSessionId,
    ProjectRuntimeSourceAnchorId, ProjectRuntimeSteeringItemId, ProjectRuntimeTurnId,
    ProjectRuntimeWorkspaceId,
};
use super::query::GraphQueryVisibility;

macro_rules! session_content_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }
    };
}

session_content_id! {
    /// Reference to content body storage owned outside the protocol envelope.
    ProjectRuntimeContentBodyRef
}

/// Token count attached to content accounting receipts.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ContentTokenCount(u32);

impl ContentTokenCount {
    pub fn new(count: u32) -> Self {
        Self(count)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl From<u32> for ContentTokenCount {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

/// Maximum token budget allowed for a compact context pack.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ContentTokenBudget(u32);

impl ContentTokenBudget {
    pub fn new(budget: u32) -> Self {
        Self(budget)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl From<u32> for ContentTokenBudget {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

/// History entries a child session may inherit.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AgentSessionHistoryLimit(u16);

impl AgentSessionHistoryLimit {
    pub fn new(limit: u16) -> Self {
        Self(limit)
    }

    pub fn as_u16(self) -> u16 {
        self.0
    }
}

impl From<u16> for AgentSessionHistoryLimit {
    fn from(value: u16) -> Self {
        Self::new(value)
    }
}

/// Session fact visible to the project runtime graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AgentSessionKind {
    Root,
    Child,
    SubAgent,
}

/// Typed session boundary fact for root, child, and sub-agent sessions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentSessionFact {
    pub project_id: ProjectRuntimeProjectId,
    pub root_session_id: ProjectRuntimeRootSessionId,
    pub session_id: ProjectRuntimeSessionId,
    pub parent_session_id: Option<ProjectRuntimeSessionId>,
    pub kind: AgentSessionKind,
    pub visibility: GraphQueryVisibility,
    pub history_limit: Option<AgentSessionHistoryLimit>,
    pub forked_from_content_id: Option<ProjectRuntimeContentId>,
    pub context_pack_id: Option<ProjectRuntimeContextPackId>,
}

impl AgentSessionFact {
    pub fn root(
        project_id: impl Into<String>,
        root_session_id: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Self {
        Self {
            project_id: ProjectRuntimeProjectId::new(project_id),
            root_session_id: ProjectRuntimeRootSessionId::new(root_session_id),
            session_id: ProjectRuntimeSessionId::new(session_id),
            parent_session_id: None,
            kind: AgentSessionKind::Root,
            visibility: GraphQueryVisibility::default(),
            history_limit: None,
            forked_from_content_id: None,
            context_pack_id: None,
        }
    }

    pub fn child(
        project_id: impl Into<String>,
        root_session_id: impl Into<String>,
        session_id: impl Into<String>,
        parent_session_id: impl Into<String>,
    ) -> Self {
        Self {
            project_id: ProjectRuntimeProjectId::new(project_id),
            root_session_id: ProjectRuntimeRootSessionId::new(root_session_id),
            session_id: ProjectRuntimeSessionId::new(session_id),
            parent_session_id: Some(ProjectRuntimeSessionId::new(parent_session_id)),
            kind: AgentSessionKind::Child,
            visibility: GraphQueryVisibility::default(),
            history_limit: None,
            forked_from_content_id: None,
            context_pack_id: None,
        }
    }

    pub fn sub_agent(
        project_id: impl Into<String>,
        root_session_id: impl Into<String>,
        session_id: impl Into<String>,
        parent_session_id: impl Into<String>,
    ) -> Self {
        Self {
            kind: AgentSessionKind::SubAgent,
            ..Self::child(project_id, root_session_id, session_id, parent_session_id)
        }
    }

    pub fn with_visibility(mut self, visibility: GraphQueryVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn with_history_limit(
        mut self,
        history_limit: impl Into<AgentSessionHistoryLimit>,
    ) -> Self {
        self.history_limit = Some(history_limit.into());
        self
    }

    pub fn with_content_fork(mut self, content_id: impl Into<String>) -> Self {
        self.forked_from_content_id = Some(ProjectRuntimeContentId::new(content_id));
        self
    }

    pub fn with_context_pack(mut self, context_pack_id: impl Into<String>) -> Self {
        self.context_pack_id = Some(ProjectRuntimeContextPackId::new(context_pack_id));
        self
    }
}

/// Role of a content node in a session graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AgentContentRole {
    System,
    User,
    Assistant,
    Tool,
    Summary,
    MemoryCandidate,
}

/// Compression state of a content node.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum AgentContentCompressionState {
    #[default]
    Raw,
    Packed,
    Compressed,
    Redacted,
}

/// Named input for building an agent content node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentContentNodeInput {
    pub project_id: ProjectRuntimeProjectId,
    pub content_id: ProjectRuntimeContentId,
    pub root_session_id: ProjectRuntimeRootSessionId,
    pub role: AgentContentRole,
    pub body_ref: ProjectRuntimeContentBodyRef,
    pub token_count: ContentTokenCount,
}

/// Content node persisted or referenced by the project runtime graph.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentContentNode {
    pub content_id: ProjectRuntimeContentId,
    pub project_id: ProjectRuntimeProjectId,
    pub workspace_id: Option<ProjectRuntimeWorkspaceId>,
    pub root_session_id: ProjectRuntimeRootSessionId,
    pub session_id: Option<ProjectRuntimeSessionId>,
    pub parent_content_id: Option<ProjectRuntimeContentId>,
    pub forked_from_content_id: Option<ProjectRuntimeContentId>,
    pub role: AgentContentRole,
    pub body_ref: ProjectRuntimeContentBodyRef,
    pub token_count: ContentTokenCount,
    pub compression_state: AgentContentCompressionState,
    #[serde(default)]
    pub source_receipts: Vec<ProjectRuntimeReceiptId>,
}

impl AgentContentNode {
    pub fn from_input(input: AgentContentNodeInput) -> Self {
        Self {
            content_id: input.content_id,
            project_id: input.project_id,
            workspace_id: None,
            root_session_id: input.root_session_id,
            session_id: None,
            parent_content_id: None,
            forked_from_content_id: None,
            role: input.role,
            body_ref: input.body_ref,
            token_count: input.token_count,
            compression_state: AgentContentCompressionState::Raw,
            source_receipts: Vec::new(),
        }
    }

    pub fn with_workspace(mut self, workspace_id: impl Into<String>) -> Self {
        self.workspace_id = Some(ProjectRuntimeWorkspaceId::new(workspace_id));
        self
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(ProjectRuntimeSessionId::new(session_id));
        self
    }

    pub fn with_parent_content(mut self, content_id: impl Into<String>) -> Self {
        self.parent_content_id = Some(ProjectRuntimeContentId::new(content_id));
        self
    }

    pub fn with_content_fork(mut self, content_id: impl Into<String>) -> Self {
        self.forked_from_content_id = Some(ProjectRuntimeContentId::new(content_id));
        self
    }

    pub fn with_compression_state(mut self, state: AgentContentCompressionState) -> Self {
        self.compression_state = state;
        self
    }

    pub fn with_source_receipt(mut self, receipt_id: impl Into<String>) -> Self {
        self.source_receipts
            .push(ProjectRuntimeReceiptId::new(receipt_id));
        self
    }
}

/// Content accounting operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ContentUsageKind {
    Prompt,
    Completion,
    ContextPack,
    Replay,
}

/// Named input for building a content usage receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentUsageReceiptInput {
    pub receipt_id: ProjectRuntimeReceiptId,
    pub project_id: ProjectRuntimeProjectId,
    pub content_id: ProjectRuntimeContentId,
    pub usage_kind: ContentUsageKind,
    pub token_count: ContentTokenCount,
}

/// Receipt recording content token accounting without embedding raw body text.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentUsageReceipt {
    pub receipt_id: ProjectRuntimeReceiptId,
    pub project_id: ProjectRuntimeProjectId,
    pub content_id: ProjectRuntimeContentId,
    pub session_id: Option<ProjectRuntimeSessionId>,
    pub usage_kind: ContentUsageKind,
    pub token_count: ContentTokenCount,
    #[serde(default)]
    pub source_receipts: Vec<ProjectRuntimeReceiptId>,
}

impl ContentUsageReceipt {
    pub fn from_input(input: ContentUsageReceiptInput) -> Self {
        Self {
            receipt_id: input.receipt_id,
            project_id: input.project_id,
            content_id: input.content_id,
            session_id: None,
            usage_kind: input.usage_kind,
            token_count: input.token_count,
            source_receipts: Vec::new(),
        }
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(ProjectRuntimeSessionId::new(session_id));
        self
    }

    pub fn with_source_receipt(mut self, receipt_id: impl Into<String>) -> Self {
        self.source_receipts
            .push(ProjectRuntimeReceiptId::new(receipt_id));
        self
    }
}

/// Compression result for a content node.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ContentCompressionStatus {
    Proposed,
    Completed,
    Skipped,
    Failed,
}

/// Receipt for content compression or summarization.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentCompressionReceipt {
    pub receipt_id: ProjectRuntimeReceiptId,
    pub source_content_id: ProjectRuntimeContentId,
    pub compressed_content_id: ProjectRuntimeContentId,
    pub status: ContentCompressionStatus,
    pub input_token_count: Option<ContentTokenCount>,
    pub output_token_count: Option<ContentTokenCount>,
    #[serde(default)]
    pub source_receipts: Vec<ProjectRuntimeReceiptId>,
}

impl ContentCompressionReceipt {
    pub fn new(
        receipt_id: impl Into<String>,
        source_content_id: impl Into<String>,
        compressed_content_id: impl Into<String>,
        status: ContentCompressionStatus,
    ) -> Self {
        Self {
            receipt_id: ProjectRuntimeReceiptId::new(receipt_id),
            source_content_id: ProjectRuntimeContentId::new(source_content_id),
            compressed_content_id: ProjectRuntimeContentId::new(compressed_content_id),
            status,
            input_token_count: None,
            output_token_count: None,
            source_receipts: Vec::new(),
        }
    }

    pub fn with_input_tokens(mut self, token_count: impl Into<ContentTokenCount>) -> Self {
        self.input_token_count = Some(token_count.into());
        self
    }

    pub fn with_output_tokens(mut self, token_count: impl Into<ContentTokenCount>) -> Self {
        self.output_token_count = Some(token_count.into());
        self
    }

    pub fn with_source_receipt(mut self, receipt_id: impl Into<String>) -> Self {
        self.source_receipts
            .push(ProjectRuntimeReceiptId::new(receipt_id));
        self
    }
}

/// Receipt for a bounded context pack handed to a child session or sub-agent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextPackReceipt {
    pub receipt_id: ProjectRuntimeReceiptId,
    pub context_pack_id: ProjectRuntimeContextPackId,
    pub root_session_id: ProjectRuntimeRootSessionId,
    pub session_id: ProjectRuntimeSessionId,
    #[serde(default)]
    pub content_ids: Vec<ProjectRuntimeContentId>,
    pub visibility: GraphQueryVisibility,
    pub token_budget: Option<ContentTokenBudget>,
    #[serde(default)]
    pub source_receipts: Vec<ProjectRuntimeReceiptId>,
}

impl ContextPackReceipt {
    pub fn new(
        receipt_id: impl Into<String>,
        context_pack_id: impl Into<String>,
        root_session_id: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Self {
        Self {
            receipt_id: ProjectRuntimeReceiptId::new(receipt_id),
            context_pack_id: ProjectRuntimeContextPackId::new(context_pack_id),
            root_session_id: ProjectRuntimeRootSessionId::new(root_session_id),
            session_id: ProjectRuntimeSessionId::new(session_id),
            content_ids: Vec::new(),
            visibility: GraphQueryVisibility::default(),
            token_budget: None,
            source_receipts: Vec::new(),
        }
    }

    pub fn with_content(mut self, content_id: impl Into<String>) -> Self {
        self.content_ids
            .push(ProjectRuntimeContentId::new(content_id));
        self
    }

    pub fn with_visibility(mut self, visibility: GraphQueryVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn with_token_budget(mut self, token_budget: impl Into<ContentTokenBudget>) -> Self {
        self.token_budget = Some(token_budget.into());
        self
    }

    pub fn with_source_receipt(mut self, receipt_id: impl Into<String>) -> Self {
        self.source_receipts
            .push(ProjectRuntimeReceiptId::new(receipt_id));
        self
    }
}

/// Item family selected or omitted while steering a turn context pack.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TurnContextItemKind {
    SessionContent,
    ProjectMemory,
    ToolCapability,
    AdditionalContext,
    Summary,
}

/// Reason an otherwise eligible item was omitted from a steered turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TurnContextOmissionReason {
    TokenBudgetExceeded,
    VisibilityDenied,
    Duplicate,
    LowerRanked,
    MissingSourceAnchor,
    ExternalProjectDenied,
    PolicyDenied,
}

/// Typed selected item id in a bounded turn context pack.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TurnContextSelectedItem {
    pub kind: TurnContextItemKind,
    pub item_id: ProjectRuntimeSteeringItemId,
}

impl TurnContextSelectedItem {
    pub fn new(kind: TurnContextItemKind, item_id: impl Into<String>) -> Self {
        Self {
            kind,
            item_id: ProjectRuntimeSteeringItemId::new(item_id),
        }
    }
}

/// Typed omitted item id and omission reason for a bounded turn context pack.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TurnContextOmittedItem {
    pub kind: TurnContextItemKind,
    pub item_id: ProjectRuntimeSteeringItemId,
    pub reason: TurnContextOmissionReason,
}

impl TurnContextOmittedItem {
    pub fn new(
        kind: TurnContextItemKind,
        item_id: impl Into<String>,
        reason: TurnContextOmissionReason,
    ) -> Self {
        Self {
            kind,
            item_id: ProjectRuntimeSteeringItemId::new(item_id),
            reason,
        }
    }
}

/// Project memory citation included in a steered turn.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectRuntimeMemoryCitation {
    pub citation_id: ProjectRuntimeMemoryCitationId,
    pub memory_id: ProjectRuntimeMemoryId,
    pub source_anchor_id: Option<ProjectRuntimeSourceAnchorId>,
    pub graph_query_receipt_id: Option<ProjectRuntimeReceiptId>,
}

impl ProjectRuntimeMemoryCitation {
    pub fn new(citation_id: impl Into<String>, memory_id: impl Into<String>) -> Self {
        Self {
            citation_id: ProjectRuntimeMemoryCitationId::new(citation_id),
            memory_id: ProjectRuntimeMemoryId::new(memory_id),
            source_anchor_id: None,
            graph_query_receipt_id: None,
        }
    }

    pub fn with_source_anchor(mut self, source_anchor_id: impl Into<String>) -> Self {
        self.source_anchor_id = Some(ProjectRuntimeSourceAnchorId::new(source_anchor_id));
        self
    }

    pub fn with_graph_query_receipt(mut self, receipt_id: impl Into<String>) -> Self {
        self.graph_query_receipt_id = Some(ProjectRuntimeReceiptId::new(receipt_id));
        self
    }
}

/// Named input for building a turn context steering receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TurnContextSteeringReceiptInput {
    pub receipt_id: ProjectRuntimeReceiptId,
    pub turn_id: ProjectRuntimeTurnId,
    pub root_session_id: ProjectRuntimeRootSessionId,
    pub session_id: ProjectRuntimeSessionId,
    pub context_pack_id: ProjectRuntimeContextPackId,
}

/// Receipt describing how a turn context pack was steered without exposing raw shards.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TurnContextSteeringReceipt {
    pub receipt_id: ProjectRuntimeReceiptId,
    pub turn_id: ProjectRuntimeTurnId,
    pub root_session_id: ProjectRuntimeRootSessionId,
    pub session_id: ProjectRuntimeSessionId,
    pub context_pack_id: ProjectRuntimeContextPackId,
    #[serde(default)]
    pub selected_items: Vec<TurnContextSelectedItem>,
    #[serde(default)]
    pub omitted_items: Vec<TurnContextOmittedItem>,
    #[serde(default)]
    pub memory_citations: Vec<ProjectRuntimeMemoryCitation>,
    #[serde(default)]
    pub source_anchor_ids: Vec<ProjectRuntimeSourceAnchorId>,
    #[serde(default)]
    pub graph_query_receipt_ids: Vec<ProjectRuntimeReceiptId>,
}

impl TurnContextSteeringReceipt {
    pub fn from_input(input: TurnContextSteeringReceiptInput) -> Self {
        Self {
            receipt_id: input.receipt_id,
            turn_id: input.turn_id,
            root_session_id: input.root_session_id,
            session_id: input.session_id,
            context_pack_id: input.context_pack_id,
            selected_items: Vec::new(),
            omitted_items: Vec::new(),
            memory_citations: Vec::new(),
            source_anchor_ids: Vec::new(),
            graph_query_receipt_ids: Vec::new(),
        }
    }

    pub fn with_selected_item(
        mut self,
        kind: TurnContextItemKind,
        item_id: impl Into<String>,
    ) -> Self {
        self.selected_items
            .push(TurnContextSelectedItem::new(kind, item_id));
        self
    }

    pub fn with_omitted_item(
        mut self,
        kind: TurnContextItemKind,
        item_id: impl Into<String>,
        reason: TurnContextOmissionReason,
    ) -> Self {
        self.omitted_items
            .push(TurnContextOmittedItem::new(kind, item_id, reason));
        self
    }

    pub fn with_memory_citation(mut self, citation: ProjectRuntimeMemoryCitation) -> Self {
        self.memory_citations.push(citation);
        self
    }

    pub fn with_source_anchor(mut self, source_anchor_id: impl Into<String>) -> Self {
        self.source_anchor_ids
            .push(ProjectRuntimeSourceAnchorId::new(source_anchor_id));
        self
    }

    pub fn with_graph_query_receipt(mut self, receipt_id: impl Into<String>) -> Self {
        self.graph_query_receipt_ids
            .push(ProjectRuntimeReceiptId::new(receipt_id));
        self
    }
}

/// Memory promotion decision for content-derived memory.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MemoryTriggerStatus {
    Candidate,
    Committed,
    Deferred,
    Rejected,
}

/// Receipt emitted when content is evaluated for project memory promotion.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemoryTriggerReceipt {
    pub receipt_id: ProjectRuntimeReceiptId,
    pub content_id: ProjectRuntimeContentId,
    pub memory_id: Option<ProjectRuntimeMemoryId>,
    pub turn_id: Option<ProjectRuntimeTurnId>,
    pub context_pack_id: Option<ProjectRuntimeContextPackId>,
    pub steering_receipt_id: Option<ProjectRuntimeReceiptId>,
    pub memory_citation_id: Option<ProjectRuntimeMemoryCitationId>,
    pub status: MemoryTriggerStatus,
    pub candidate_score: Option<GraphQueryScoreBasisPoints>,
    #[serde(default)]
    pub source_anchor_ids: Vec<ProjectRuntimeSourceAnchorId>,
    #[serde(default)]
    pub graph_query_receipt_ids: Vec<ProjectRuntimeReceiptId>,
    #[serde(default)]
    pub source_receipts: Vec<ProjectRuntimeReceiptId>,
}

impl MemoryTriggerReceipt {
    pub fn new(
        receipt_id: impl Into<String>,
        content_id: impl Into<String>,
        status: MemoryTriggerStatus,
    ) -> Self {
        Self {
            receipt_id: ProjectRuntimeReceiptId::new(receipt_id),
            content_id: ProjectRuntimeContentId::new(content_id),
            memory_id: None,
            turn_id: None,
            context_pack_id: None,
            steering_receipt_id: None,
            memory_citation_id: None,
            status,
            candidate_score: None,
            source_anchor_ids: Vec::new(),
            graph_query_receipt_ids: Vec::new(),
            source_receipts: Vec::new(),
        }
    }

    pub fn with_memory(mut self, memory_id: impl Into<String>) -> Self {
        self.memory_id = Some(ProjectRuntimeMemoryId::new(memory_id));
        self
    }

    pub fn with_candidate_score(
        mut self,
        candidate_score: impl Into<GraphQueryScoreBasisPoints>,
    ) -> Self {
        self.candidate_score = Some(candidate_score.into());
        self
    }

    pub fn with_turn(mut self, turn_id: impl Into<String>) -> Self {
        self.turn_id = Some(ProjectRuntimeTurnId::new(turn_id));
        self
    }

    pub fn with_context_pack(mut self, context_pack_id: impl Into<String>) -> Self {
        self.context_pack_id = Some(ProjectRuntimeContextPackId::new(context_pack_id));
        self
    }

    pub fn with_steering_receipt(mut self, receipt_id: impl Into<String>) -> Self {
        self.steering_receipt_id = Some(ProjectRuntimeReceiptId::new(receipt_id));
        self
    }

    pub fn with_memory_citation(mut self, citation_id: impl Into<String>) -> Self {
        self.memory_citation_id = Some(ProjectRuntimeMemoryCitationId::new(citation_id));
        self
    }

    pub fn with_source_anchor(mut self, source_anchor_id: impl Into<String>) -> Self {
        self.source_anchor_ids
            .push(ProjectRuntimeSourceAnchorId::new(source_anchor_id));
        self
    }

    pub fn with_graph_query_receipt(mut self, receipt_id: impl Into<String>) -> Self {
        self.graph_query_receipt_ids
            .push(ProjectRuntimeReceiptId::new(receipt_id));
        self
    }

    pub fn with_source_receipt(mut self, receipt_id: impl Into<String>) -> Self {
        self.source_receipts
            .push(ProjectRuntimeReceiptId::new(receipt_id));
        self
    }
}
