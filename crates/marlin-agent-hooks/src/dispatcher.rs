//! Dispatches registered `HookRuntime` handlers and returns protocol-owned hook receipts.

use std::{collections::BTreeMap, sync::Arc};

use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use marlin_agent_protocol::{
    HookAgentScope, HookConfigurationVersion, HookDecisionContext, HookDispatchPolicyReceipt,
    HookDispatchPolicyReceiptInput, HookDispatchSelectionInput, HookDispatchSelectionReceipt,
    HookEventName, HookExecutionMode, HookHandlerType, HookMatcherStrategy, HookMatcherToken,
    HookPolicyDecision, HookPolicyDecisionReason, HookPolicyDecisionReceipt,
    HookPolicyDynamicActionApplicationReceipt, HookPolicyExtension, HookPolicyMode,
    HookRegistryUpdateKind, HookRegistryUpdateReceipt, HookRunId, HookRunStatus, HookRunSummary,
    HookScope, HookSelectedCandidateInput, HookSelectionCandidateReceipt, HookSelectionSkipReason,
    HookSkippedCandidateInput, HookSource, HookSourcePath, HookTrustStatus,
};
use marlin_agent_runtime::{HookRuntime, RuntimeContext, TokioAgentRuntime, observability};
use tracing::Instrument;

use crate::dynamic_actions::{
    HookRegistrationCatalog, HookRegistryActionTarget, apply_dynamic_policy_actions,
    apply_dynamic_registry_actions, attach_dynamic_actions,
};

/// Runtime shape accepted by hook registrations.
pub type RegisteredHookRuntime =
    dyn HookRuntime<Request = HookInvocation, Output = HookRunSummary> + Send + Sync + 'static;

/// Runtime-independent finalizer for extension-owned hook policy decisions.
pub type RegisteredHookPolicyFinalizer = dyn HookDispatchPolicyFinalizer + Send + Sync + 'static;

/// Runtime catalog that resolves extension-requested hook registrations.
pub type RegisteredHookRegistrationCatalog =
    dyn HookRegistrationCatalog<HookRegistration> + Send + Sync + 'static;

/// Runtime task boundary for hook dispatch execution.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct HookDispatchTaskOwner;

impl HookDispatchTaskOwner {
    /// Runtime task boundary: owns spawned hook handler execution and returns the join handle.
    fn spawn(
        handler: Arc<RegisteredHookRuntime>,
        invocation: HookInvocation,
        context: RuntimeContext,
        span: tracing::Span,
    ) -> tokio::task::JoinHandle<HookRunSummary> {
        tokio::spawn(async move { handler.run_hook(invocation, context).await }.instrument(span))
    }
}

/// Input passed to registered hook runtimes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookInvocation {
    pub event_name: HookEventName,
    pub agent_scope: HookAgentScope,
    pub decision_context: HookDecisionContext,
    pub message: Option<String>,
}

impl HookInvocation {
    /// Creates a hook invocation for an event.
    pub fn new(event_name: HookEventName) -> Self {
        Self {
            event_name,
            agent_scope: HookAgentScope::Any,
            decision_context: HookDecisionContext::default(),
            message: None,
        }
    }

    /// Assigns the invoking agent runtime scope.
    pub fn with_agent_scope(mut self, agent_scope: HookAgentScope) -> Self {
        self.agent_scope = agent_scope;
        self
    }

    /// Assigns typed context facts for policy selection and extension decisions.
    pub fn with_decision_context(mut self, decision_context: HookDecisionContext) -> Self {
        self.decision_context = decision_context;
        self
    }

    /// Adds session and execution facts from a runtime context to the hook decision context.
    pub fn with_runtime_context(mut self, context: &RuntimeContext) -> Self {
        let session = context.session();
        let mut decision_context = self
            .decision_context
            .with_session_id(session.session_id().as_str().to_owned())
            .with_agent_lineage_node(format!(
                "root_session:{}",
                session.root_session_id().as_str()
            ));
        if let Some(parent_session_id) = session.parent_session_id() {
            decision_context = decision_context
                .with_agent_lineage_node(format!("parent_session:{}", parent_session_id.as_str()));
        }
        if let Some(execution) = context.execution_identity() {
            decision_context = decision_context
                .with_workspace_state(format!("run_id={}", execution.run_id()))
                .with_workspace_state(format!("graph_id={}", execution.graph_id()));
        }
        self.decision_context = decision_context;
        self
    }

    /// Adds a human-readable message to the invocation.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Input passed to a hook policy finalizer after Rust policy evaluation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookDispatchPolicyFinalizerInput {
    pub invocation: HookInvocation,
    pub policy_receipt: HookDispatchPolicyReceipt,
}

/// Extension boundary that can finalize hook policy receipts.
pub trait HookDispatchPolicyFinalizer {
    fn finalize(&self, input: HookDispatchPolicyFinalizerInput) -> HookDispatchPolicyReceipt;
}

/// Registered hook handler and its ordering metadata.
#[derive(Clone)]
pub struct HookRegistration {
    pub id: String,
    pub event_name: HookEventName,
    pub handler_type: HookHandlerType,
    pub execution_mode: HookExecutionMode,
    pub scope: HookScope,
    pub agent_scope: HookAgentScope,
    pub source_path: Option<HookSourcePath>,
    pub source: HookSource,
    pub trust: HookTrustStatus,
    pub enabled: bool,
    pub display_order: i64,
    handler: Arc<RegisteredHookRuntime>,
}

impl HookRegistration {
    /// Creates a hook registration with conservative default metadata.
    pub fn new<H>(
        id: impl Into<String>,
        event_name: HookEventName,
        handler_type: HookHandlerType,
        handler: Arc<H>,
    ) -> Self
    where
        H: HookRuntime<Request = HookInvocation, Output = HookRunSummary>,
    {
        let handler: Arc<RegisteredHookRuntime> = handler;
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
            enabled: true,
            display_order: 0,
            handler,
        }
    }

    /// Sets the hook execution mode.
    pub fn with_execution_mode(mut self, execution_mode: HookExecutionMode) -> Self {
        self.execution_mode = execution_mode;
        self
    }

    /// Sets the hook scope.
    pub fn with_scope(mut self, scope: HookScope) -> Self {
        self.scope = scope;
        self
    }

    /// Sets the agent runtime scope for this hook registration.
    pub fn with_agent_scope(mut self, agent_scope: HookAgentScope) -> Self {
        self.agent_scope = agent_scope;
        self
    }

    /// Sets the hook source path.
    pub fn with_source_path(mut self, source_path: impl Into<HookSourcePath>) -> Self {
        self.source_path = Some(source_path.into());
        self
    }

    /// Sets the hook source.
    pub fn with_source(mut self, source: HookSource) -> Self {
        self.source = source;
        self
    }

    /// Sets the hook trust status.
    pub fn with_trust(mut self, trust: HookTrustStatus) -> Self {
        self.trust = trust;
        self
    }

    /// Enables or disables this hook registration.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets the hook display order. Lower values run first.
    pub fn with_display_order(mut self, display_order: i64) -> Self {
        self.display_order = display_order;
        self
    }

    fn run(
        &self,
        context: RuntimeContext,
        invocation: HookInvocation,
    ) -> tokio::task::JoinHandle<HookRunSummary> {
        let handler = self.handler.clone();
        let span = hook_run_span(self);
        HookDispatchTaskOwner::spawn(handler, invocation, context, span)
    }
}

/// Ordered registry of hook registrations.
#[derive(Clone)]
pub struct HookRegistry {
    registrations: Vec<HookRegistration>,
    event_index: HookEventIndex,
}

impl HookRegistry {
    /// Creates an empty hook registry.
    pub fn new() -> Self {
        Self {
            registrations: Vec::new(),
            event_index: HookEventIndex::default(),
        }
    }

    /// Registers one hook handler.
    pub fn register(&mut self, registration: HookRegistration) {
        self.registrations.push(registration);
        self.sort();
        self.rebuild_event_index();
    }

    /// Registers one hook handler and returns a dynamic update receipt.
    pub fn register_with_receipt(
        &mut self,
        registration: HookRegistration,
        configuration_version: Option<HookConfigurationVersion>,
    ) -> HookRegistryUpdateReceipt {
        let receipt = hook_registry_update_receipt(
            &registration,
            HookRegistryUpdateKind::Registered,
            self.registrations.len() + 1,
            configuration_version,
        );
        self.register(registration);
        receipt
    }

    /// Returns a new registry containing one additional hook registration.
    pub fn with_registration(mut self, registration: HookRegistration) -> Self {
        self.register(registration);
        self
    }

    /// Removes one hook registration by identifier.
    pub fn unregister(
        &mut self,
        hook_id: &HookRunId,
        configuration_version: Option<HookConfigurationVersion>,
    ) -> Option<HookRegistryUpdateReceipt> {
        let index = self
            .registrations
            .iter()
            .position(|registration| registration.id == hook_id.as_str())?;
        let registration = self.registrations.remove(index);
        self.rebuild_event_index();
        Some(hook_registry_update_receipt(
            &registration,
            HookRegistryUpdateKind::Unregistered,
            self.registrations.len(),
            configuration_version,
        ))
    }

    /// Enables or disables one registered hook by identifier.
    pub fn set_enabled(
        &mut self,
        hook_id: &HookRunId,
        enabled: bool,
        configuration_version: Option<HookConfigurationVersion>,
    ) -> Option<HookRegistryUpdateReceipt> {
        let registration_count = self.registrations.len();
        let registration = self
            .registrations
            .iter_mut()
            .find(|registration| registration.id == hook_id.as_str())?;
        registration.enabled = enabled;
        let receipt = hook_registry_update_receipt(
            registration,
            if enabled {
                HookRegistryUpdateKind::Enabled
            } else {
                HookRegistryUpdateKind::Disabled
            },
            registration_count,
            configuration_version,
        );
        self.rebuild_event_index();
        Some(receipt)
    }

    /// Returns all registrations in dispatch order.
    pub fn registrations(&self) -> &[HookRegistration] {
        &self.registrations
    }

    fn matching(
        &self,
        invocation: &HookInvocation,
    ) -> (Vec<HookRegistration>, HookDispatchSelectionReceipt) {
        let indexed_match = self.event_index.matching_indexes(&invocation.event_name);
        let candidates = indexed_match
            .indexes
            .iter()
            .filter_map(|index| self.registrations.get(*index))
            .map(|registration| hook_selection_candidate(registration, invocation))
            .collect::<Vec<_>>();
        let registrations = candidates
            .iter()
            .zip(indexed_match.indexes.iter())
            .filter(|(candidate, _)| candidate.selected)
            .filter_map(|(_, index)| self.registrations.get(*index))
            .cloned()
            .collect::<Vec<_>>();
        let selection = HookDispatchSelectionReceipt::new(HookDispatchSelectionInput {
            event_name: invocation.event_name.clone(),
            invocation_agent_scope: invocation.agent_scope.clone(),
            decision_context: invocation.decision_context.clone(),
            matcher_strategy: indexed_match.strategy,
            matched_tokens: indexed_match.matched_tokens,
            candidates,
        });
        (registrations, selection)
    }

    fn sort(&mut self) {
        self.registrations
            .sort_by_key(|registration| registration.display_order);
    }

    fn rebuild_event_index(&mut self) {
        self.event_index = HookEventIndex::from_registrations(&self.registrations);
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl HookRegistryActionTarget<HookRegistration> for HookRegistry {
    fn register_dynamic_hook(
        &mut self,
        registration: HookRegistration,
    ) -> HookRegistryUpdateReceipt {
        self.register_with_receipt(registration, None)
    }

    fn unregister_dynamic_hook(
        &mut self,
        hook_id: &HookRunId,
    ) -> Option<HookRegistryUpdateReceipt> {
        self.unregister(hook_id, None)
    }
}

#[derive(Clone, Debug, Default)]
struct HookEventIndex {
    automaton: Option<AhoCorasick>,
    registrations_by_pattern: Vec<Vec<usize>>,
}

#[derive(Clone, Debug)]
struct HookEventIndexMatch {
    strategy: HookMatcherStrategy,
    matched_tokens: Vec<HookMatcherToken>,
    indexes: Vec<usize>,
}

impl HookEventIndex {
    fn from_registrations(registrations: &[HookRegistration]) -> Self {
        let registrations_by_token = hook_registrations_by_event_token(registrations);
        if registrations_by_token.is_empty() {
            return Self::default();
        }

        let patterns = registrations_by_token.keys().cloned().collect::<Vec<_>>();
        let registrations_by_pattern = patterns
            .iter()
            .map(|pattern| {
                registrations_by_token
                    .get(pattern)
                    .cloned()
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();
        let automaton = AhoCorasickBuilder::new()
            .ascii_case_insensitive(false)
            .build(&patterns)
            .expect("hook event tokens are generated from stable enum variants");

        Self {
            automaton: Some(automaton),
            registrations_by_pattern,
        }
    }

    fn matching_indexes(&self, event_name: &HookEventName) -> HookEventIndexMatch {
        let Some(automaton) = &self.automaton else {
            return HookEventIndexMatch {
                strategy: HookMatcherStrategy::LinearScan,
                matched_tokens: Vec::new(),
                indexes: Vec::new(),
            };
        };
        let event_token = hook_event_token(event_name);
        let matches = automaton
            .find_iter(event_token.as_str())
            .collect::<Vec<_>>();
        let matched_tokens = matches
            .iter()
            .map(|_| HookMatcherToken::new(event_token.clone()))
            .collect::<Vec<_>>();
        let indexes = matches
            .into_iter()
            .flat_map(|matched| {
                self.registrations_by_pattern[matched.pattern().as_usize()]
                    .iter()
                    .copied()
            })
            .collect::<Vec<_>>();
        HookEventIndexMatch {
            strategy: HookMatcherStrategy::AhoCorasickEventIndex,
            matched_tokens,
            indexes,
        }
    }
}

fn hook_selection_candidate(
    registration: &HookRegistration,
    invocation: &HookInvocation,
) -> HookSelectionCandidateReceipt {
    if !registration.enabled {
        HookSelectionCandidateReceipt::skipped(HookSkippedCandidateInput {
            hook_id: registration.id.clone().into(),
            event_name: registration.event_name.clone(),
            registration_agent_scope: registration.agent_scope.clone(),
            invocation_agent_scope: invocation.agent_scope.clone(),
            skip_reason: HookSelectionSkipReason::Disabled,
        })
    } else if registration.event_name != invocation.event_name {
        HookSelectionCandidateReceipt::skipped(HookSkippedCandidateInput {
            hook_id: registration.id.clone().into(),
            event_name: registration.event_name.clone(),
            registration_agent_scope: registration.agent_scope.clone(),
            invocation_agent_scope: invocation.agent_scope.clone(),
            skip_reason: HookSelectionSkipReason::EventMismatch,
        })
    } else if !registration
        .agent_scope
        .matches_invocation(&invocation.agent_scope)
    {
        HookSelectionCandidateReceipt::skipped(HookSkippedCandidateInput {
            hook_id: registration.id.clone().into(),
            event_name: registration.event_name.clone(),
            registration_agent_scope: registration.agent_scope.clone(),
            invocation_agent_scope: invocation.agent_scope.clone(),
            skip_reason: HookSelectionSkipReason::AgentScopeMismatch,
        })
    } else {
        HookSelectionCandidateReceipt::selected(HookSelectedCandidateInput {
            hook_id: registration.id.clone().into(),
            event_name: registration.event_name.clone(),
            registration_agent_scope: registration.agent_scope.clone(),
            invocation_agent_scope: invocation.agent_scope.clone(),
        })
    }
}

fn hook_registry_update_receipt(
    registration: &HookRegistration,
    kind: HookRegistryUpdateKind,
    registration_count: usize,
    configuration_version: Option<HookConfigurationVersion>,
) -> HookRegistryUpdateReceipt {
    HookRegistryUpdateReceipt {
        hook_id: registration.id.clone().into(),
        kind,
        agent_scope: registration.agent_scope.clone(),
        enabled: registration.enabled,
        registration_count,
        configuration_version,
    }
}

fn hook_registrations_by_event_token(
    registrations: &[HookRegistration],
) -> BTreeMap<String, Vec<usize>> {
    let mut registrations_by_token = BTreeMap::<String, Vec<usize>>::new();
    registrations
        .iter()
        .enumerate()
        .for_each(|(index, registration)| {
            registrations_by_token
                .entry(hook_event_token(&registration.event_name))
                .or_default()
                .push(index);
        });
    registrations_by_token
}

fn hook_event_token(event_name: &HookEventName) -> String {
    format!("|{}|", hook_event_key(event_name))
}

fn hook_event_key(event_name: &HookEventName) -> &'static str {
    match event_name {
        HookEventName::PreToolUse => "PreToolUse",
        HookEventName::PermissionRequest => "PermissionRequest",
        HookEventName::PostToolUse => "PostToolUse",
        HookEventName::PreCompact => "PreCompact",
        HookEventName::PostCompact => "PostCompact",
        HookEventName::SessionStart => "SessionStart",
        HookEventName::UserPromptSubmit => "UserPromptSubmit",
        HookEventName::SubAgentStart => "SubAgentStart",
        HookEventName::SubAgentStop => "SubAgentStop",
        HookEventName::Stop => "Stop",
    }
}

/// Dispatch policy that decides whether selected hook registrations may run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookDispatchPolicy {
    mode: HookPolicyMode,
    extension: HookPolicyExtension,
}

impl HookDispatchPolicy {
    /// Creates a dispatch policy from a protocol policy mode.
    pub fn from_mode(mode: HookPolicyMode) -> Self {
        Self {
            mode,
            extension: HookPolicyExtension::none(),
        }
    }

    /// Creates an observe-only policy that records decisions without blocking.
    pub fn observe_only() -> Self {
        Self::from_mode(HookPolicyMode::ObserveOnly)
    }

    /// Creates a policy that only allows trusted or managed hook sources.
    pub fn enforce_trusted() -> Self {
        Self::from_mode(HookPolicyMode::EnforceTrusted)
    }

    /// Returns the protocol policy mode.
    pub fn mode(&self) -> &HookPolicyMode {
        &self.mode
    }

    /// Returns the optional policy extension boundary.
    pub fn extension(&self) -> &HookPolicyExtension {
        &self.extension
    }

    /// Returns a dispatch policy with a different extension boundary.
    pub fn with_extension(mut self, extension: HookPolicyExtension) -> Self {
        self.extension = extension;
        self
    }

    fn evaluate(
        &self,
        invocation: &HookInvocation,
        registrations: &[HookRegistration],
    ) -> HookDispatchPolicyReceipt {
        HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
            event_name: invocation.event_name.clone(),
            invocation_agent_scope: invocation.agent_scope.clone(),
            decision_context: invocation.decision_context.clone(),
            mode: self.mode.clone(),
            extension: self.extension.clone(),
            actions: Vec::new(),
            decisions: registrations
                .iter()
                .map(|registration| self.evaluate_registration(registration))
                .collect(),
        })
    }

    fn evaluate_registration(&self, registration: &HookRegistration) -> HookPolicyDecisionReceipt {
        let (decision, reason) = self.registration_decision(registration);
        HookPolicyDecisionReceipt {
            hook_id: registration.id.clone().into(),
            event_name: registration.event_name.clone(),
            handler_type: registration.handler_type.clone(),
            scope: registration.scope.clone(),
            agent_scope: registration.agent_scope.clone(),
            source_path: registration.source_path.clone(),
            source: registration.source.clone(),
            trust: registration.trust.clone(),
            decision,
            reason,
        }
    }

    fn registration_decision(
        &self,
        registration: &HookRegistration,
    ) -> (HookPolicyDecision, HookPolicyDecisionReason) {
        match (&self.mode, &registration.trust) {
            (_, HookTrustStatus::Managed) => (
                HookPolicyDecision::Allowed,
                HookPolicyDecisionReason::ManagedSource,
            ),
            (_, HookTrustStatus::Trusted) => (
                HookPolicyDecision::Allowed,
                HookPolicyDecisionReason::TrustedSource,
            ),
            (HookPolicyMode::ObserveOnly, HookTrustStatus::Untrusted) => (
                HookPolicyDecision::Allowed,
                HookPolicyDecisionReason::UntrustedAllowedByObserveOnly,
            ),
            (HookPolicyMode::ObserveOnly, HookTrustStatus::Modified) => (
                HookPolicyDecision::Allowed,
                HookPolicyDecisionReason::ModifiedAllowedByObserveOnly,
            ),
            (HookPolicyMode::EnforceTrusted, HookTrustStatus::Untrusted) => (
                HookPolicyDecision::Rejected,
                HookPolicyDecisionReason::UntrustedRejected,
            ),
            (HookPolicyMode::EnforceTrusted, HookTrustStatus::Modified) => (
                HookPolicyDecision::Rejected,
                HookPolicyDecisionReason::ModifiedRejected,
            ),
        }
    }
}

impl Default for HookDispatchPolicy {
    fn default() -> Self {
        Self::observe_only()
    }
}

/// Dispatches hook invocations through a registry.
#[derive(Clone, Default)]
pub struct HookDispatcher {
    registry: HookRegistry,
    policy: HookDispatchPolicy,
    policy_finalizer: Option<Arc<RegisteredHookPolicyFinalizer>>,
    registration_catalog: Option<Arc<RegisteredHookRegistrationCatalog>>,
}

impl HookDispatcher {
    /// Creates a dispatcher from an ordered registry.
    pub fn new(registry: HookRegistry) -> Self {
        Self {
            registry,
            policy: HookDispatchPolicy::default(),
            policy_finalizer: None,
            registration_catalog: None,
        }
    }

    /// Returns the dispatcher registry.
    pub fn registry(&self) -> &HookRegistry {
        &self.registry
    }

    /// Returns the dispatcher policy.
    pub fn policy(&self) -> &HookDispatchPolicy {
        &self.policy
    }

    /// Returns the optional extension-owned policy finalizer.
    pub fn policy_finalizer(&self) -> Option<&RegisteredHookPolicyFinalizer> {
        self.policy_finalizer.as_deref()
    }

    /// Returns the optional runtime registration catalog.
    pub fn registration_catalog(&self) -> Option<&RegisteredHookRegistrationCatalog> {
        self.registration_catalog.as_deref()
    }

    /// Returns a dispatcher with a different policy.
    pub fn with_policy(mut self, policy: HookDispatchPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Returns a dispatcher with an extension-owned policy finalizer.
    pub fn with_policy_finalizer<F>(mut self, finalizer: Arc<F>) -> Self
    where
        F: HookDispatchPolicyFinalizer + Send + Sync + 'static,
    {
        let finalizer: Arc<RegisteredHookPolicyFinalizer> = finalizer;
        self.policy_finalizer = Some(finalizer);
        self
    }

    /// Returns a dispatcher with an extension-requested registration catalog.
    pub fn with_registration_catalog<C>(mut self, catalog: Arc<C>) -> Self
    where
        C: HookRegistrationCatalog<HookRegistration> + Send + Sync + 'static,
    {
        let catalog: Arc<RegisteredHookRegistrationCatalog> = catalog;
        self.registration_catalog = Some(catalog);
        self
    }

    /// Runs matching hooks and returns their receipts.
    pub async fn dispatch(
        &self,
        runtime: &TokioAgentRuntime,
        invocation: HookInvocation,
    ) -> HookDispatchReport {
        self.dispatch_with_context(runtime.context(), invocation)
            .await
    }

    /// Runs matching hooks with an existing runtime context and returns their receipts.
    pub async fn dispatch_with_context(
        &self,
        context: RuntimeContext,
        invocation: HookInvocation,
    ) -> HookDispatchReport {
        let prepared = self.prepare_dispatch(invocation);
        let runs = run_permitted_hook_registrations(
            context,
            prepared.invocation.clone(),
            prepared.registrations,
        )
        .instrument(observability::hook_dispatch_span(
            &prepared.invocation.event_name,
            prepared.registration_count,
        ))
        .await;

        HookDispatchReport {
            event_name: prepared.invocation.event_name,
            selection: prepared.selection,
            policy: prepared.policy,
            applied_actions: prepared.applied_actions,
            runs,
        }
    }

    fn prepare_dispatch(&self, invocation: HookInvocation) -> PreparedHookDispatch {
        let mut dispatch_registry = self.registry.clone();
        let (registrations, _) = dispatch_registry.matching(&invocation);
        let policy = self.policy.evaluate(&invocation, &registrations);
        let finalized_policy = self.finalize_policy(&invocation, policy);
        let actions = finalized_policy.actions.clone();
        let registry_application = apply_dynamic_registry_actions(
            &mut dispatch_registry,
            &actions,
            self.registration_catalog.as_deref(),
        );
        let mut applied_actions = registry_application.receipts;
        let (registrations, selection, policy) = if registry_application.changed {
            let (registrations, selection) = dispatch_registry.matching(&invocation);
            let policy =
                attach_dynamic_actions(self.policy.evaluate(&invocation, &registrations), actions);
            (registrations, selection, policy)
        } else {
            let (registrations, selection) = dispatch_registry.matching(&invocation);
            (registrations, selection, finalized_policy)
        };
        let policy_application = apply_dynamic_policy_actions(invocation.message.clone(), policy);
        let invocation = HookInvocation {
            message: policy_application.message,
            ..invocation
        };
        let policy = policy_application.policy;
        applied_actions.extend(policy_application.receipts);
        let registrations = permitted_registrations(registrations, &policy);

        PreparedHookDispatch {
            registration_count: registrations.len(),
            invocation,
            selection,
            policy,
            applied_actions,
            registrations,
        }
    }

    fn finalize_policy(
        &self,
        invocation: &HookInvocation,
        policy_receipt: HookDispatchPolicyReceipt,
    ) -> HookDispatchPolicyReceipt {
        self.policy_finalizer
            .as_ref()
            .map_or(policy_receipt.clone(), |finalizer| {
                finalizer.finalize(HookDispatchPolicyFinalizerInput {
                    invocation: invocation.clone(),
                    policy_receipt,
                })
            })
    }
}

struct PreparedHookDispatch {
    registration_count: usize,
    invocation: HookInvocation,
    selection: HookDispatchSelectionReceipt,
    policy: HookDispatchPolicyReceipt,
    applied_actions: Vec<HookPolicyDynamicActionApplicationReceipt>,
    registrations: Vec<HookRegistration>,
}

/// Receipt collection for one hook event dispatch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookDispatchReport {
    pub event_name: HookEventName,
    pub selection: HookDispatchSelectionReceipt,
    pub policy: HookDispatchPolicyReceipt,
    pub applied_actions: Vec<HookPolicyDynamicActionApplicationReceipt>,
    pub runs: Vec<HookRunSummary>,
}

impl HookDispatchReport {
    /// Returns true when every hook completed successfully.
    pub fn is_success(&self) -> bool {
        self.policy.is_success()
            && self
                .runs
                .iter()
                .all(|run| run.status == HookRunStatus::Completed)
    }
}

async fn run_permitted_hook_registrations(
    context: RuntimeContext,
    invocation: HookInvocation,
    registrations: Vec<HookRegistration>,
) -> Vec<HookRunSummary> {
    let mut runs = Vec::new();
    let mut async_runs = Vec::new();

    for registration in registrations {
        match registration.execution_mode {
            HookExecutionMode::Sync => {
                let span = hook_run_span(&registration);
                let summary = registration
                    .handler
                    .run_hook(invocation.clone(), context.child_context())
                    .instrument(span)
                    .await;
                runs.push(apply_registration_metadata(&registration, summary));
            }
            HookExecutionMode::Async => {
                let task = registration.run(context.child_context(), invocation.clone());
                async_runs.push((registration, task));
            }
        }
    }

    for (registration, task) in async_runs {
        let summary = match task.await {
            Ok(summary) => summary,
            Err(error) => {
                tracing::warn!(
                    hook_id = %registration.id,
                    hook_event = ?registration.event_name,
                    error = %error,
                    "async hook task join failed"
                );
                failed_join_summary(&registration, &invocation, error.to_string())
            }
        };
        runs.push(apply_registration_metadata(&registration, summary));
    }

    runs
}

fn permitted_registrations(
    registrations: Vec<HookRegistration>,
    policy: &HookDispatchPolicyReceipt,
) -> Vec<HookRegistration> {
    registrations
        .into_iter()
        .zip(policy.decisions.iter())
        .filter(|(_, decision)| decision.decision == HookPolicyDecision::Allowed)
        .map(|(registration, _)| registration)
        .collect()
}

fn apply_registration_metadata(
    registration: &HookRegistration,
    mut summary: HookRunSummary,
) -> HookRunSummary {
    summary.event_name = registration.event_name.clone();
    summary.handler_type = registration.handler_type.clone();
    summary.execution_mode = registration.execution_mode.clone();
    summary.scope = registration.scope.clone();
    summary.agent_scope = registration.agent_scope.clone();
    summary.source_path = registration.source_path.clone();
    summary.source = registration.source.clone();
    summary.trust = registration.trust.clone();
    summary.display_order = registration.display_order;
    summary
}

fn hook_run_span(registration: &HookRegistration) -> tracing::Span {
    observability::hook_run_span(
        observability::HookRegistrationId::borrowed(&registration.id),
        &registration.event_name,
        &registration.execution_mode,
        &registration.handler_type,
    )
}

fn failed_join_summary(
    registration: &HookRegistration,
    invocation: &HookInvocation,
    status_message: String,
) -> HookRunSummary {
    let mut summary = HookRunSummary::running(
        registration.id.clone(),
        invocation.event_name.clone(),
        registration.handler_type.clone(),
    );
    summary.status = HookRunStatus::Failed;
    summary.status_message = Some(status_message);
    summary
}
