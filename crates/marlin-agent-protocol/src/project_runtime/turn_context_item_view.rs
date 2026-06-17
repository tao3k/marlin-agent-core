//! Compact item views rendered from runtime-read memory and tool context.

use serde::{Deserialize, Serialize};

use super::ids::{
    ProjectRuntimeBackendRequirementId, ProjectRuntimeContextPackId,
    ProjectRuntimeIsolationRequirementId, ProjectRuntimeMemoryCitationId, ProjectRuntimeReceiptId,
    ProjectRuntimeSourceAnchorId, ProjectRuntimeSteeringItemId, ProjectRuntimeToolCapabilityId,
};
use super::query::{
    ProjectMemoryContextFact, ProjectMemoryContextPack, ProjectRuntimeToolCapabilityCard,
};
use super::session_content::{ProjectRuntimeMemoryCitation, TurnContextItemKind};

/// Agent-facing compact view for one selected context item.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TurnContextItemView {
    pub kind: TurnContextItemKind,
    pub item_id: ProjectRuntimeSteeringItemId,
    pub summary: String,
    pub memory_citation_id: Option<ProjectRuntimeMemoryCitationId>,
    #[serde(default)]
    pub source_anchor_ids: Vec<ProjectRuntimeSourceAnchorId>,
    #[serde(default)]
    pub graph_query_receipt_ids: Vec<ProjectRuntimeReceiptId>,
    #[serde(default)]
    pub required_receipt_ids: Vec<ProjectRuntimeReceiptId>,
    #[serde(default)]
    pub required_capability_ids: Vec<ProjectRuntimeToolCapabilityId>,
    #[serde(default)]
    pub isolation_requirement_ids: Vec<ProjectRuntimeIsolationRequirementId>,
    #[serde(default)]
    pub backend_requirement_ids: Vec<ProjectRuntimeBackendRequirementId>,
}

impl TurnContextItemView {
    pub fn new(
        kind: TurnContextItemKind,
        item_id: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            item_id: ProjectRuntimeSteeringItemId::new(item_id),
            summary: summary.into(),
            memory_citation_id: None,
            source_anchor_ids: Vec::new(),
            graph_query_receipt_ids: Vec::new(),
            required_receipt_ids: Vec::new(),
            required_capability_ids: Vec::new(),
            isolation_requirement_ids: Vec::new(),
            backend_requirement_ids: Vec::new(),
        }
    }

    pub fn with_memory_citation(mut self, citation_id: impl Into<String>) -> Self {
        self.memory_citation_id = Some(ProjectRuntimeMemoryCitationId::new(citation_id));
        self
    }

    pub fn with_source_anchor(mut self, source_anchor_id: impl Into<String>) -> Self {
        push_unique(
            &mut self.source_anchor_ids,
            ProjectRuntimeSourceAnchorId::new(source_anchor_id),
        );
        self
    }

    pub fn with_graph_query_receipt(mut self, receipt_id: impl Into<String>) -> Self {
        push_unique(
            &mut self.graph_query_receipt_ids,
            ProjectRuntimeReceiptId::new(receipt_id),
        );
        self
    }

    pub fn with_required_receipt(mut self, receipt_id: impl Into<String>) -> Self {
        push_unique(
            &mut self.required_receipt_ids,
            ProjectRuntimeReceiptId::new(receipt_id),
        );
        self
    }

    pub fn with_required_capability(mut self, capability_id: impl Into<String>) -> Self {
        push_unique(
            &mut self.required_capability_ids,
            ProjectRuntimeToolCapabilityId::new(capability_id),
        );
        self
    }

    pub fn with_isolation_requirement(mut self, requirement_id: impl Into<String>) -> Self {
        push_unique(
            &mut self.isolation_requirement_ids,
            ProjectRuntimeIsolationRequirementId::new(requirement_id),
        );
        self
    }

    pub fn with_backend_requirement(mut self, requirement_id: impl Into<String>) -> Self {
        push_unique(
            &mut self.backend_requirement_ids,
            ProjectRuntimeBackendRequirementId::new(requirement_id),
        );
        self
    }
}

/// Receipt proving selected memory/tool context has been rendered into item views.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TurnContextItemViewReceipt {
    pub receipt_id: ProjectRuntimeReceiptId,
    pub context_pack_id: ProjectRuntimeContextPackId,
    pub steering_receipt_id: Option<ProjectRuntimeReceiptId>,
    #[serde(default)]
    pub item_views: Vec<TurnContextItemView>,
    #[serde(default)]
    pub source_anchor_ids: Vec<ProjectRuntimeSourceAnchorId>,
    #[serde(default)]
    pub graph_query_receipt_ids: Vec<ProjectRuntimeReceiptId>,
}

impl TurnContextItemViewReceipt {
    pub fn new(receipt_id: impl Into<String>, context_pack_id: impl Into<String>) -> Self {
        Self {
            receipt_id: ProjectRuntimeReceiptId::new(receipt_id),
            context_pack_id: ProjectRuntimeContextPackId::new(context_pack_id),
            steering_receipt_id: None,
            item_views: Vec::new(),
            source_anchor_ids: Vec::new(),
            graph_query_receipt_ids: Vec::new(),
        }
    }

    pub fn from_memory_context_pack(
        receipt_id: impl Into<String>,
        context_pack: &ProjectMemoryContextPack,
        memory_citations: &[ProjectRuntimeMemoryCitation],
    ) -> Self {
        Self::new(receipt_id, context_pack.context_pack_id.as_str())
            .with_memory_context_pack(context_pack, memory_citations)
    }

    pub fn with_steering_receipt(mut self, receipt_id: impl Into<String>) -> Self {
        self.steering_receipt_id = Some(ProjectRuntimeReceiptId::new(receipt_id));
        self
    }

    pub fn with_memory_context_pack(
        mut self,
        context_pack: &ProjectMemoryContextPack,
        memory_citations: &[ProjectRuntimeMemoryCitation],
    ) -> Self {
        for fact in &context_pack.facts {
            self = self.with_memory_context_fact(fact, memory_citations);
        }
        for receipt_id in &context_pack.source_receipts {
            self = self.with_graph_query_receipt(receipt_id.as_str());
        }
        self
    }

    pub fn with_memory_context_fact(
        self,
        fact: &ProjectMemoryContextFact,
        memory_citations: &[ProjectRuntimeMemoryCitation],
    ) -> Self {
        let item_id = fact
            .graph_match
            .memory_id
            .as_ref()
            .map(|id| id.as_str())
            .or_else(|| {
                fact.graph_match
                    .source_anchor_id
                    .as_ref()
                    .map(|id| id.as_str())
            })
            .unwrap_or(fact.claim.as_str());
        let mut item_view =
            TurnContextItemView::new(TurnContextItemKind::ProjectMemory, item_id, &fact.claim);
        if let Some(source_anchor_id) = &fact.graph_match.source_anchor_id {
            item_view = item_view.with_source_anchor(source_anchor_id.as_str());
        }
        if let Some(receipt_id) = &fact.graph_match.receipt_id {
            item_view = item_view.with_graph_query_receipt(receipt_id.as_str());
        }
        if let Some(citation) = matching_memory_citation(fact, memory_citations) {
            item_view = item_view.with_memory_citation(citation.citation_id.as_str());
            if let Some(source_anchor_id) = &citation.source_anchor_id {
                item_view = item_view.with_source_anchor(source_anchor_id.as_str());
            }
            if let Some(receipt_id) = &citation.graph_query_receipt_id {
                item_view = item_view.with_graph_query_receipt(receipt_id.as_str());
            }
        }
        self.with_item_view(item_view)
    }

    pub fn with_tool_capability_card(self, card: &ProjectRuntimeToolCapabilityCard) -> Self {
        let item_id = card
            .graph_match
            .tool_capability_id
            .as_ref()
            .map(|id| id.as_str())
            .or_else(|| {
                card.graph_match
                    .source_anchor_id
                    .as_ref()
                    .map(|id| id.as_str())
            })
            .unwrap_or(card.graph_match.summary.as_str());
        let mut item_view = TurnContextItemView::new(
            TurnContextItemKind::ToolCapability,
            item_id,
            &card.graph_match.summary,
        );
        if let Some(source_anchor_id) = &card.graph_match.source_anchor_id {
            item_view = item_view.with_source_anchor(source_anchor_id.as_str());
        }
        if let Some(receipt_id) = &card.graph_match.receipt_id {
            item_view = item_view.with_graph_query_receipt(receipt_id.as_str());
        }
        for receipt_id in &card.required_receipt_ids {
            item_view = item_view.with_required_receipt(receipt_id.as_str());
        }
        for capability_id in &card.required_capability_ids {
            item_view = item_view.with_required_capability(capability_id.as_str());
        }
        for requirement_id in &card.isolation_requirement_ids {
            item_view = item_view.with_isolation_requirement(requirement_id.as_str());
        }
        for requirement_id in &card.backend_requirement_ids {
            item_view = item_view.with_backend_requirement(requirement_id.as_str());
        }
        self.with_item_view(item_view)
    }

    pub fn with_item_view(mut self, item_view: TurnContextItemView) -> Self {
        for source_anchor_id in &item_view.source_anchor_ids {
            push_unique(&mut self.source_anchor_ids, source_anchor_id.clone());
        }
        for receipt_id in &item_view.graph_query_receipt_ids {
            push_unique(&mut self.graph_query_receipt_ids, receipt_id.clone());
        }
        self.item_views.push(item_view);
        self
    }

    pub fn with_graph_query_receipt(mut self, receipt_id: impl Into<String>) -> Self {
        push_unique(
            &mut self.graph_query_receipt_ids,
            ProjectRuntimeReceiptId::new(receipt_id),
        );
        self
    }
}

fn matching_memory_citation<'a>(
    fact: &ProjectMemoryContextFact,
    memory_citations: &'a [ProjectRuntimeMemoryCitation],
) -> Option<&'a ProjectRuntimeMemoryCitation> {
    fact.graph_match.memory_id.as_ref().and_then(|memory_id| {
        memory_citations
            .iter()
            .find(|citation| citation.memory_id == *memory_id)
    })
}

fn push_unique<T: Eq>(items: &mut Vec<T>, item: T) {
    if !items.contains(&item) {
        items.push(item);
    }
}
