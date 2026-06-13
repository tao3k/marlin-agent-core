use marlin_agent_hooks::{HookInvocation, HookRegistration};
use marlin_agent_protocol::{
    HookHandlerType, HookOutputEntry, HookOutputEntryKind, HookRunSummary,
};
use marlin_agent_runtime::{HookRuntime, RuntimeContext, RuntimeFuture};

#[derive(Clone, Debug)]
pub(super) struct SummaryHook {
    id: &'static str,
}

impl SummaryHook {
    pub(super) fn new(id: &'static str) -> Self {
        Self { id }
    }
}

impl HookRuntime for SummaryHook {
    type Request = HookInvocation;
    type Output = HookRunSummary;

    fn run_hook(
        &self,
        request: Self::Request,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let id = self.id;
        Box::pin(async move {
            let mut summary =
                HookRunSummary::running(id, request.event_name, HookHandlerType::Command);
            if let Some(message) = request.message {
                summary = summary.with_entry(HookOutputEntry {
                    kind: HookOutputEntryKind::Context,
                    text: message,
                });
            }
            summary.completed()
        })
    }
}

pub(super) fn summary_hook_registration(
    id: &'static str,
    event_name: marlin_agent_protocol::HookEventName,
    handler_type: HookHandlerType,
    run_id: &'static str,
) -> HookRegistration {
    HookRegistration::new(
        id,
        event_name,
        handler_type,
        std::sync::Arc::new(SummaryHook::new(run_id)),
    )
}
