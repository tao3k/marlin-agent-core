//! Hook protocol contracts for runtime interception and observation.

use serde::{Deserialize, Serialize};

/// Runtime event that can trigger configured hook handlers.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HookEventName {
    PreToolUse,
    PermissionRequest,
    PostToolUse,
    PreCompact,
    PostCompact,
    SessionStart,
    UserPromptSubmit,
    SubAgentStart,
    SubAgentStop,
    Stop,
}

/// Kind of handler used to execute a hook.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HookHandlerType {
    Command,
    Prompt,
    Agent,
}

/// Scheduling mode for a hook handler.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HookExecutionMode {
    Sync,
    Async,
}

/// Lifetime scope for hook state.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HookScope {
    Thread,
    Turn,
}

/// Configuration source that provided a hook.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HookSource {
    System,
    User,
    Project,
    SessionFlags,
    Plugin,
    Unknown,
}

/// Trust state assigned to the hook source.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HookTrustStatus {
    Managed,
    Untrusted,
    Trusted,
    Modified,
}

/// Agent runtime scope a hook registration applies to.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HookAgentScope {
    Any,
    RootAgent,
    SubAgent,
    CustomAgent,
    CustomerAgent,
    ForkedAgent,
    IsolatedAgent,
    PersistentAgent,
}

impl HookAgentScope {
    pub fn matches_invocation(&self, invocation_scope: &Self) -> bool {
        self == &Self::Any || self == invocation_scope
    }
}

/// Runtime status for one hook invocation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HookRunStatus {
    Running,
    Completed,
    Failed,
    Blocked,
    Stopped,
}

/// Structured hook output entry category.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HookOutputEntryKind {
    Warning,
    Stop,
    Feedback,
    Context,
    Error,
}

/// Structured output emitted by one hook run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookOutputEntry {
    pub kind: HookOutputEntryKind,
    pub text: String,
}

impl HookOutputEntry {
    pub fn new(kind: HookOutputEntryKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
        }
    }
}

/// Stable identifier for one hook run.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HookRunId(String);

impl HookRunId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for HookRunId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for HookRunId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Source path string recorded in a hook protocol receipt.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HookSourcePath(String);

impl HookSourcePath {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for HookSourcePath {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for HookSourcePath {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Millisecond timestamp in hook protocol receipts.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HookTimestampMs(i64);

impl HookTimestampMs {
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    pub fn as_i64(self) -> i64 {
        self.0
    }
}

impl From<i64> for HookTimestampMs {
    fn from(value: i64) -> Self {
        Self::new(value)
    }
}

/// Millisecond duration in hook protocol receipts.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HookDurationMs(i64);

impl HookDurationMs {
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    pub fn as_i64(self) -> i64 {
        self.0
    }
}

impl From<i64> for HookDurationMs {
    fn from(value: i64) -> Self {
        Self::new(value)
    }
}

/// Matcher strategy used to select hook registrations for one dispatch.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HookMatcherStrategy {
    AhoCorasickEventIndex,
    LinearScan,
}

/// Stable matcher token observed while selecting hook registrations.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HookMatcherToken(String);

impl HookMatcherToken {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for HookMatcherToken {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for HookMatcherToken {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Version label for a dynamically loaded hook configuration.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HookConfigurationVersion(String);

impl HookConfigurationVersion {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for HookConfigurationVersion {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for HookConfigurationVersion {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Runtime used by a hook policy extension.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum HookPolicyExtensionKind {
    #[default]
    None,
    GerbilScheme,
}

/// Gerbil module used as an extension policy entrypoint.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HookSchemeModule(String);

impl HookSchemeModule {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for HookSchemeModule {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for HookSchemeModule {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Gerbil procedure used as an extension policy entrypoint.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HookSchemeProcedure(String);

impl HookSchemeProcedure {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for HookSchemeProcedure {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for HookSchemeProcedure {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Optional policy extension boundary for complex hook decisions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HookPolicyExtension {
    pub kind: HookPolicyExtensionKind,
    pub module: Option<HookSchemeModule>,
    pub procedure: Option<HookSchemeProcedure>,
}

impl HookPolicyExtension {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn gerbil_scheme(
        module: impl Into<HookSchemeModule>,
        procedure: impl Into<HookSchemeProcedure>,
    ) -> Self {
        Self {
            kind: HookPolicyExtensionKind::GerbilScheme,
            module: Some(module.into()),
            procedure: Some(procedure.into()),
        }
    }
}

impl Default for HookPolicyExtension {
    fn default() -> Self {
        Self {
            kind: HookPolicyExtensionKind::None,
            module: None,
            procedure: None,
        }
    }
}

/// Typed reason why a hook registration candidate was not selected.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HookSelectionSkipReason {
    Disabled,
    AgentScopeMismatch,
    EventMismatch,
}

/// Selection receipt for one hook registration candidate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookSelectionCandidateReceipt {
    pub hook_id: HookRunId,
    pub event_name: HookEventName,
    pub registration_agent_scope: HookAgentScope,
    pub invocation_agent_scope: HookAgentScope,
    pub selected: bool,
    pub skip_reason: Option<HookSelectionSkipReason>,
}

/// Named input for a selected hook registration candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookSelectedCandidateInput {
    pub hook_id: HookRunId,
    pub event_name: HookEventName,
    pub registration_agent_scope: HookAgentScope,
    pub invocation_agent_scope: HookAgentScope,
}

/// Named input for a skipped hook registration candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookSkippedCandidateInput {
    pub hook_id: HookRunId,
    pub event_name: HookEventName,
    pub registration_agent_scope: HookAgentScope,
    pub invocation_agent_scope: HookAgentScope,
    pub skip_reason: HookSelectionSkipReason,
}

impl HookSelectionCandidateReceipt {
    pub fn selected(input: HookSelectedCandidateInput) -> Self {
        Self {
            hook_id: input.hook_id,
            event_name: input.event_name,
            registration_agent_scope: input.registration_agent_scope,
            invocation_agent_scope: input.invocation_agent_scope,
            selected: true,
            skip_reason: None,
        }
    }

    pub fn skipped(input: HookSkippedCandidateInput) -> Self {
        Self {
            hook_id: input.hook_id,
            event_name: input.event_name,
            registration_agent_scope: input.registration_agent_scope,
            invocation_agent_scope: input.invocation_agent_scope,
            selected: false,
            skip_reason: Some(input.skip_reason),
        }
    }
}

/// Selection receipt emitted before hook handlers run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookDispatchSelectionReceipt {
    pub event_name: HookEventName,
    pub invocation_agent_scope: HookAgentScope,
    pub matcher_strategy: HookMatcherStrategy,
    pub matched_tokens: Vec<HookMatcherToken>,
    pub candidate_count: usize,
    pub selected_count: usize,
    pub candidates: Vec<HookSelectionCandidateReceipt>,
}

/// Named input for constructing a hook dispatch selection receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookDispatchSelectionInput {
    pub event_name: HookEventName,
    pub invocation_agent_scope: HookAgentScope,
    pub matcher_strategy: HookMatcherStrategy,
    pub matched_tokens: Vec<HookMatcherToken>,
    pub candidates: Vec<HookSelectionCandidateReceipt>,
}

impl HookDispatchSelectionReceipt {
    pub fn new(input: HookDispatchSelectionInput) -> Self {
        let HookDispatchSelectionInput {
            event_name,
            invocation_agent_scope,
            matcher_strategy,
            matched_tokens,
            candidates,
        } = input;
        let candidate_count = candidates.len();
        let selected_count = candidates
            .iter()
            .filter(|candidate| candidate.selected)
            .count();
        Self {
            event_name,
            invocation_agent_scope,
            matcher_strategy,
            matched_tokens,
            candidate_count,
            selected_count,
            candidates,
        }
    }
}

/// Policy mode used while deciding whether selected hooks may execute.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HookPolicyMode {
    ObserveOnly,
    EnforceTrusted,
}

/// Execution decision produced by hook policy evaluation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HookPolicyDecision {
    Allowed,
    Rejected,
}

/// Typed reason attached to a hook policy decision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HookPolicyDecisionReason {
    ManagedSource,
    TrustedSource,
    UntrustedAllowedByObserveOnly,
    ModifiedAllowedByObserveOnly,
    UntrustedRejected,
    ModifiedRejected,
    ExtensionAllowed,
    ExtensionRejected,
    ExtensionDeferred,
}

/// Dynamic hook action target emitted by an extension policy.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HookPolicyDynamicActionTarget(String);

impl HookPolicyDynamicActionTarget {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for HookPolicyDynamicActionTarget {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for HookPolicyDynamicActionTarget {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Dynamic hook rewrite value emitted by an extension policy.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HookPolicyDynamicActionReplacement(String);

impl HookPolicyDynamicActionReplacement {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for HookPolicyDynamicActionReplacement {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for HookPolicyDynamicActionReplacement {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Dynamic hook action reason emitted by an extension policy.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HookPolicyDynamicActionReason(String);

impl HookPolicyDynamicActionReason {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for HookPolicyDynamicActionReason {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for HookPolicyDynamicActionReason {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Dynamic hook action kind emitted by an extension policy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HookPolicyDynamicActionKind {
    Deny,
    Rewrite,
    Register,
    Unregister,
    Defer,
}

/// Dynamic hook action emitted by an extension policy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookPolicyDynamicAction {
    pub kind: HookPolicyDynamicActionKind,
    #[serde(default)]
    pub target: Option<HookPolicyDynamicActionTarget>,
    #[serde(default)]
    pub replacement: Option<HookPolicyDynamicActionReplacement>,
    #[serde(default)]
    pub reason: Option<HookPolicyDynamicActionReason>,
}

/// Runtime status for applying one dynamic hook policy action.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HookPolicyDynamicActionApplicationStatus {
    Applied,
    Ignored,
    Failed,
}

/// Runtime effect produced by applying one dynamic hook policy action.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HookPolicyDynamicActionApplicationEffect {
    DecisionRejected,
    DispatchDeferred,
    InvocationRewritten,
    RegistrationRegistered,
    RegistrationUnregistered,
    Noop,
}

/// Runtime reason for one dynamic hook policy action application.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HookPolicyDynamicActionApplicationReason {
    TargetMatched,
    TargetNotMatched,
    MissingTarget,
    MissingReplacement,
    UnsupportedTarget,
    CatalogUnavailable,
    CatalogMiss,
    RegistryMiss,
    CatalogResolved,
    RegistryUpdated,
}

/// Receipt proving how the runtime handled one dynamic hook policy action.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookPolicyDynamicActionApplicationReceipt {
    pub action: HookPolicyDynamicAction,
    pub status: HookPolicyDynamicActionApplicationStatus,
    pub effect: HookPolicyDynamicActionApplicationEffect,
    pub reason: HookPolicyDynamicActionApplicationReason,
    pub target_hook_id: Option<HookRunId>,
    pub registry_update: Option<HookRegistryUpdateReceipt>,
}

/// Policy decision receipt for one selected hook registration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookPolicyDecisionReceipt {
    pub hook_id: HookRunId,
    pub event_name: HookEventName,
    pub handler_type: HookHandlerType,
    pub scope: HookScope,
    pub agent_scope: HookAgentScope,
    pub source_path: Option<HookSourcePath>,
    pub source: HookSource,
    pub trust: HookTrustStatus,
    pub decision: HookPolicyDecision,
    pub reason: HookPolicyDecisionReason,
}

/// Policy receipt emitted before hook handlers execute.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookDispatchPolicyReceipt {
    pub event_name: HookEventName,
    pub invocation_agent_scope: HookAgentScope,
    pub mode: HookPolicyMode,
    pub extension: HookPolicyExtension,
    pub evaluated_count: usize,
    pub allowed_count: usize,
    pub rejected_count: usize,
    pub actions: Vec<HookPolicyDynamicAction>,
    pub decisions: Vec<HookPolicyDecisionReceipt>,
}

/// Named input for building a hook dispatch policy receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookDispatchPolicyReceiptInput {
    pub event_name: HookEventName,
    pub invocation_agent_scope: HookAgentScope,
    pub mode: HookPolicyMode,
    pub extension: HookPolicyExtension,
    pub actions: Vec<HookPolicyDynamicAction>,
    pub decisions: Vec<HookPolicyDecisionReceipt>,
}

impl HookDispatchPolicyReceipt {
    pub fn new(input: HookDispatchPolicyReceiptInput) -> Self {
        let evaluated_count = input.decisions.len();
        let allowed_count = input
            .decisions
            .iter()
            .filter(|decision| decision.decision == HookPolicyDecision::Allowed)
            .count();
        let rejected_count = evaluated_count - allowed_count;
        Self {
            event_name: input.event_name,
            invocation_agent_scope: input.invocation_agent_scope,
            mode: input.mode,
            extension: input.extension,
            evaluated_count,
            allowed_count,
            rejected_count,
            actions: input.actions,
            decisions: input.decisions,
        }
    }

    pub fn is_success(&self) -> bool {
        self.rejected_count == 0
    }
}

/// Dynamic mutation kind applied to a hook registry.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HookRegistryUpdateKind {
    Registered,
    Unregistered,
    Enabled,
    Disabled,
    Reloaded,
}

/// Receipt emitted after a dynamic hook registry mutation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookRegistryUpdateReceipt {
    pub hook_id: HookRunId,
    pub kind: HookRegistryUpdateKind,
    pub agent_scope: HookAgentScope,
    pub enabled: bool,
    pub registration_count: usize,
    pub configuration_version: Option<HookConfigurationVersion>,
}

/// Receipt emitted after a hook `TOML` configuration reload.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookConfigurationReloadReceipt {
    pub configuration_version: Option<HookConfigurationVersion>,
    pub policy_mode: HookPolicyMode,
    pub policy_extension: HookPolicyExtension,
    pub registration_default_scope: HookScope,
    pub registration_default_agent_scope: HookAgentScope,
    pub registration_count: usize,
}

/// Protocol receipt for one hook run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookRunSummary {
    pub id: HookRunId,
    pub event_name: HookEventName,
    pub handler_type: HookHandlerType,
    pub execution_mode: HookExecutionMode,
    pub scope: HookScope,
    pub agent_scope: HookAgentScope,
    pub source_path: Option<HookSourcePath>,
    pub source: HookSource,
    pub trust: HookTrustStatus,
    pub display_order: i64,
    pub status: HookRunStatus,
    pub status_message: Option<String>,
    pub started_at_ms: HookTimestampMs,
    pub completed_at_ms: Option<HookTimestampMs>,
    pub duration_ms: Option<HookDurationMs>,
    pub entries: Vec<HookOutputEntry>,
}

impl HookRunSummary {
    pub fn running(
        id: impl Into<HookRunId>,
        event_name: HookEventName,
        handler_type: HookHandlerType,
    ) -> Self {
        Self {
            id: id.into(),
            event_name,
            handler_type,
            execution_mode: HookExecutionMode::Sync,
            scope: HookScope::Turn,
            agent_scope: HookAgentScope::Any,
            source_path: None,
            source: HookSource::Unknown,
            trust: HookTrustStatus::Untrusted,
            display_order: 0,
            status: HookRunStatus::Running,
            status_message: None,
            started_at_ms: HookTimestampMs::new(0),
            completed_at_ms: None,
            duration_ms: None,
            entries: Vec::new(),
        }
    }

    pub fn completed(mut self) -> Self {
        self.status = HookRunStatus::Completed;
        self
    }

    pub fn with_entry(mut self, entry: HookOutputEntry) -> Self {
        self.entries.push(entry);
        self
    }
}
