// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Schedule sub-agents with bounded parallelism and per-worktree receipts.

use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use tokio::task::{Id as TaskId, JoinError, JoinSet};
use tracing::Instrument;

/// Hard upper bound on concurrent working-copy sub-agent fanout tasks.
pub const MAX_WORKING_COPY_FANOUT_PARALLELISM: usize = 64;

use super::TokioAgentRuntime;
use crate::{
    observability,
    tokio_runtime::{
        RuntimeFanoutJoinPolicy, RuntimeFanoutOutput, RuntimeFanoutReceipt, RuntimeFanoutResult,
        RuntimeFanoutTaskReceipt, RuntimeOperationId, RuntimeTask, SubAgentRuntime,
        WorkingCopySubAgentFanoutItem,
    },
};

impl TokioAgentRuntime {
    pub fn spawn_sub_agents_with_working_copy_environments<A>(
        &self,
        sub_agent: Arc<A>,
        fanout: impl IntoIterator<Item = WorkingCopySubAgentFanoutItem<A::Input>>,
        max_parallelism: usize,
    ) -> RuntimeTask<RuntimeFanoutResult<A::Output>>
    where
        A: SubAgentRuntime,
    {
        self.spawn_sub_agents_with_working_copy_environments_with_policy(
            sub_agent,
            fanout,
            RuntimeFanoutJoinPolicy::bounded(max_parallelism),
        )
    }

    pub fn spawn_sub_agents_with_working_copy_environments_with_policy<A>(
        &self,
        sub_agent: Arc<A>,
        fanout: impl IntoIterator<Item = WorkingCopySubAgentFanoutItem<A::Input>>,
        policy: RuntimeFanoutJoinPolicy,
    ) -> RuntimeTask<RuntimeFanoutResult<A::Output>>
    where
        A: SubAgentRuntime,
    {
        let runtime = self.clone();
        let fanout = fanout.into_iter().collect::<Vec<_>>();

        self.spawn_scoped(
            self.session.session_id(),
            RuntimeOperationId::new("subagent.fanout"),
            Self::run_sub_agent_fanout_with_policy(runtime, sub_agent, fanout, policy),
            observability::runtime_sub_agent_span(),
        )
    }

    fn effective_working_copy_fanout_parallelism(
        policy: &RuntimeFanoutJoinPolicy,
        item_count: usize,
    ) -> usize {
        policy
            .max_parallelism()
            .unwrap_or_else(|| item_count.max(1))
            .clamp(1, MAX_WORKING_COPY_FANOUT_PARALLELISM)
    }

    async fn run_sub_agent_fanout_with_policy<A>(
        runtime: Self,
        sub_agent: Arc<A>,
        fanout: Vec<WorkingCopySubAgentFanoutItem<A::Input>>,
        policy: RuntimeFanoutJoinPolicy,
    ) -> RuntimeFanoutResult<A::Output>
    where
        A: SubAgentRuntime,
    {
        WorkingCopySubAgentFanoutRunner::new(runtime, sub_agent, fanout, policy)
            .run()
            .await
    }
}

struct WorkingCopySubAgentFanoutRunner<A>
where
    A: SubAgentRuntime,
{
    runtime: TokioAgentRuntime,
    sub_agent: Arc<A>,
    pending: VecDeque<(usize, WorkingCopySubAgentFanoutItem<A::Input>)>,
    tasks: JoinSet<(usize, A::Output)>,
    active_indices: HashMap<TaskId, usize>,
    outputs: Vec<(usize, A::Output)>,
    receipt: RuntimeFanoutReceipt,
    policy: RuntimeFanoutJoinPolicy,
    effective_parallelism: usize,
}

impl<A> WorkingCopySubAgentFanoutRunner<A>
where
    A: SubAgentRuntime,
{
    fn new(
        runtime: TokioAgentRuntime,
        sub_agent: Arc<A>,
        fanout: Vec<WorkingCopySubAgentFanoutItem<A::Input>>,
        policy: RuntimeFanoutJoinPolicy,
    ) -> Self {
        let effective_parallelism =
            TokioAgentRuntime::effective_working_copy_fanout_parallelism(&policy, fanout.len());
        Self {
            runtime,
            sub_agent,
            pending: fanout.into_iter().enumerate().collect(),
            tasks: JoinSet::new(),
            active_indices: HashMap::new(),
            outputs: Vec::new(),
            receipt: RuntimeFanoutReceipt::new(policy.clone()),
            policy,
            effective_parallelism,
        }
    }

    async fn run(mut self) -> RuntimeFanoutResult<A::Output> {
        while self.has_work() {
            self.spawn_ready_tasks();
            match self.record_next_join().await {
                WorkingCopyFanoutStep::Continue => {}
                WorkingCopyFanoutStep::Cancelled(receipt) => return Err(receipt),
            }
        }

        self.finish()
    }

    fn has_work(&self) -> bool {
        !self.pending.is_empty() || !self.tasks.is_empty()
    }

    fn spawn_ready_tasks(&mut self) {
        while self.tasks.len() < self.effective_parallelism {
            let Some((index, item)) = self.pending.pop_front() else {
                break;
            };
            let (input, environment, working_copy_receipt) = item.into_parts();
            let sub_agent = Arc::clone(&self.sub_agent);
            let context = self
                .runtime
                .context()
                .child_context_with_environment(environment)
                .with_working_copy_receipt(working_copy_receipt);
            let Ok(guard) = self.runtime.task_tracker.admit(
                context.session().session_id(),
                RuntimeOperationId::new("subagent.fanout.item"),
            ) else {
                self.receipt
                    .push_task_receipt(RuntimeFanoutTaskReceipt::cancelled_before_start(index));
                continue;
            };
            let handle = self.tasks.spawn(
                async move {
                    let _guard = guard;
                    (index, sub_agent.run_sub_agent(input, context).await)
                }
                .instrument(observability::runtime_sub_agent_span()),
            );
            self.active_indices.insert(handle.id(), index);
        }
    }

    async fn record_next_join(&mut self) -> WorkingCopyFanoutStep {
        let Some(joined) = self.tasks.join_next_with_id().await else {
            return WorkingCopyFanoutStep::Continue;
        };

        match joined {
            Ok((task_id, (index, output))) => {
                self.active_indices.remove(&task_id);
                self.record_completed_task(index, output)
            }
            Err(error) => {
                let input_index = self.active_indices.remove(&error.id());
                self.record_join_error(input_index, error).await
            }
        }
    }

    fn record_completed_task(&mut self, index: usize, output: A::Output) -> WorkingCopyFanoutStep {
        self.receipt
            .push_task_receipt(RuntimeFanoutTaskReceipt::completed(index));
        self.outputs.push((index, output));
        WorkingCopyFanoutStep::Continue
    }

    async fn record_join_error(
        &mut self,
        input_index: Option<usize>,
        error: JoinError,
    ) -> WorkingCopyFanoutStep {
        let receipt = match input_index {
            Some(index) => RuntimeFanoutTaskReceipt::join_error_for_input(index, error.to_string()),
            None => RuntimeFanoutTaskReceipt::join_error(error.to_string()),
        };
        self.receipt.push_task_receipt(receipt);
        if !self.policy.cancel_on_first_error() {
            return WorkingCopyFanoutStep::Continue;
        }

        WorkingCopyFanoutStep::Cancelled(self.cancel_remaining_work().await)
    }

    async fn cancel_remaining_work(&mut self) -> RuntimeFanoutReceipt {
        self.tasks.abort_all();
        while self.tasks.join_next_with_id().await.is_some() {}
        self.active_indices.clear();
        while let Some((index, _)) = self.pending.pop_front() {
            self.receipt
                .push_task_receipt(RuntimeFanoutTaskReceipt::cancelled_before_start(index));
        }
        self.take_receipt()
    }

    fn finish(mut self) -> RuntimeFanoutResult<A::Output> {
        if self.receipt.has_join_errors() {
            return Err(self.receipt);
        }
        if self.policy.preserve_input_order() {
            self.outputs.sort_by_key(|(index, _)| *index);
        }
        Ok(RuntimeFanoutOutput::new(
            self.outputs.into_iter().map(|(_, output)| output).collect(),
            self.receipt,
        ))
    }

    fn take_receipt(&mut self) -> RuntimeFanoutReceipt {
        std::mem::replace(
            &mut self.receipt,
            RuntimeFanoutReceipt::new(self.policy.clone()),
        )
    }
}

enum WorkingCopyFanoutStep {
    Continue,
    Cancelled(RuntimeFanoutReceipt),
}
