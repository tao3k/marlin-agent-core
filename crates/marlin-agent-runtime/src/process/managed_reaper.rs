// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Managed child graceful termination executor and ledger/span finalization.

use std::{
    io,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use process_wrap::tokio::ChildWrapper;

use super::ledger::{ManagedProcessLedgerEntry, ProcessGroupId};
use super::registry::RuntimeProcessRegistryHandle;
use super::shutdown::{
    ProcessGracefulShutdownPolicy, ProcessShutdownHook, ProcessTerminationStage,
    is_process_group_alive, signal_process_group_kill, signal_process_group_term,
    wait_for_process_group_exit,
};
use super::types::{
    RuntimeProcessExitStatus, RuntimeProcessObservationTimestampMs, RuntimeProcessStatus,
};

pub(super) struct GracefulTerminationContext {
    pub(super) registry: RuntimeProcessRegistryHandle,
    pub(super) pid: u32,
    pub(super) pgid: Option<ProcessGroupId>,
    pub(super) policy: ProcessGracefulShutdownPolicy,
    pub(super) hook: Option<ProcessShutdownHook>,
    pub(super) ledger_entry: ManagedProcessLedgerEntry,
    pub(super) span: tracing::Span,
    pub(super) spawned_at: Instant,
}

pub(super) struct GracefulTerminationOutcome {
    pub(super) status: RuntimeProcessExitStatus,
    pub(super) escalated_to_sigkill: bool,
    pub(super) termination_stage: ProcessTerminationStage,
}

pub(super) async fn run_graceful_termination(
    child: &mut Box<dyn ChildWrapper>,
    ctx: &mut GracefulTerminationContext,
    observed_at_ms: u64,
) -> io::Result<GracefulTerminationOutcome> {
    let pgid_u32 = ctx.pgid.map(ProcessGroupId::as_u32);
    let mut escalated_to_sigkill = false;

    if let Some(hook) = ctx.hook.take() {
        record_stage(
            ctx,
            ProcessTerminationStage::CooperativeShutdownRequested,
            observed_at_ms,
        );
        if hook.invoke(ctx.policy.cooperative_timeout).await.is_ok()
            && !ctx.policy.graceful_timeout.is_zero()
            && let Ok(result) =
                tokio::time::timeout(ctx.policy.graceful_timeout, child.wait()).await
        {
            return complete_after_leader_exit(
                ctx.pid,
                pgid_u32,
                &ctx.policy,
                result?,
                &mut escalated_to_sigkill,
            )
            .await;
        }
    }

    if let Ok(Some(status)) = child.try_wait() {
        return complete_after_leader_exit(
            ctx.pid,
            pgid_u32,
            &ctx.policy,
            status,
            &mut escalated_to_sigkill,
        )
        .await;
    }

    if !ctx.policy.graceful_timeout.is_zero() {
        if child.id().is_some() {
            record_stage(ctx, ProcessTerminationStage::GracefulTermSent, now_millis());
            let _ = signal_process_group_term(ctx.pid, pgid_u32);
        }
        if let Ok(result) = tokio::time::timeout(ctx.policy.graceful_timeout, child.wait()).await {
            return complete_after_leader_exit(
                ctx.pid,
                pgid_u32,
                &ctx.policy,
                result?,
                &mut escalated_to_sigkill,
            )
            .await;
        }
    }

    if ctx.policy.escalate_to_sigkill {
        escalated_to_sigkill = true;
        ctx.ledger_entry.escalated_to_sigkill = true;
        record_stage(
            ctx,
            ProcessTerminationStage::ForceKillEscalated,
            now_millis(),
        );
        if child.id().is_some() {
            let _ = child.start_kill();
        }
        let _ = signal_process_group_kill(ctx.pid, pgid_u32);
    }

    let exit = RuntimeProcessExitStatus::from(child.wait().await?);
    wait_for_process_group_exit(pgid_u32.unwrap_or(ctx.pid)).await?;
    let termination_stage = terminal_stage_for_exit(&exit, escalated_to_sigkill);
    Ok(GracefulTerminationOutcome {
        status: exit,
        escalated_to_sigkill,
        termination_stage,
    })
}

pub(super) fn terminal_stage_for_exit(
    exit: &RuntimeProcessExitStatus,
    escalated_to_sigkill: bool,
) -> ProcessTerminationStage {
    if escalated_to_sigkill {
        ProcessTerminationStage::ForceKillEscalated
    } else if exit.success() {
        ProcessTerminationStage::ExitedCleanly
    } else {
        ProcessTerminationStage::ExitedFailed
    }
}

pub(super) struct LedgerFinalization<'a> {
    pub(super) exit: &'a RuntimeProcessExitStatus,
    pub(super) stage: ProcessTerminationStage,
    pub(super) escalated_to_sigkill: bool,
    pub(super) observed_at_ms: u64,
}

pub(super) fn finalize_ledger_and_span(
    registry: &RuntimeProcessRegistryHandle,
    entry: &mut ManagedProcessLedgerEntry,
    span: &tracing::Span,
    spawned_at: Instant,
    finalization: LedgerFinalization<'_>,
) {
    let ts = RuntimeProcessObservationTimestampMs::new(finalization.observed_at_ms);
    entry.status = if finalization.exit.success() {
        RuntimeProcessStatus::Finished
    } else {
        RuntimeProcessStatus::Failed
    };
    entry.termination_stage = finalization.stage;
    entry.last_observed_at_ms = ts;
    entry.ended_at_ms = Some(ts);
    entry.exit_code = finalization.exit.code();
    entry.escalated_to_sigkill = finalization.escalated_to_sigkill;
    registry.persist_ledger_entry(entry);

    let duration_ms = u64::try_from(spawned_at.elapsed().as_millis()).unwrap_or(u64::MAX);
    span.record("termination_stage", finalization.stage.as_str());
    span.record("escalated_to_sigkill", finalization.escalated_to_sigkill);
    span.record("duration_ms", duration_ms);
    if let Some(code) = finalization.exit.code() {
        span.record("exit_code", code);
    }
}

fn record_stage(
    ctx: &mut GracefulTerminationContext,
    stage: ProcessTerminationStage,
    observed_at_ms: u64,
) {
    ctx.ledger_entry.status = RuntimeProcessStatus::CleanupRequested;
    ctx.ledger_entry.termination_stage = stage;
    ctx.ledger_entry.last_observed_at_ms =
        RuntimeProcessObservationTimestampMs::new(observed_at_ms);
    ctx.ledger_entry.cleanup_attempts = ctx.ledger_entry.cleanup_attempts.saturating_add(1);
    ctx.registry.persist_ledger_entry(&ctx.ledger_entry);
}

async fn complete_after_leader_exit(
    pid: u32,
    pgid: Option<u32>,
    policy: &ProcessGracefulShutdownPolicy,
    status: std::process::ExitStatus,
    escalated_to_sigkill: &mut bool,
) -> io::Result<GracefulTerminationOutcome> {
    drain_group_after_leader_exit(pid, pgid, policy, escalated_to_sigkill).await?;
    let exit = RuntimeProcessExitStatus::from(status);
    let termination_stage = terminal_stage_for_exit(&exit, *escalated_to_sigkill);
    Ok(GracefulTerminationOutcome {
        status: exit,
        escalated_to_sigkill: *escalated_to_sigkill,
        termination_stage,
    })
}

async fn drain_group_after_leader_exit(
    pid: u32,
    pgid: Option<u32>,
    policy: &ProcessGracefulShutdownPolicy,
    escalated_to_sigkill: &mut bool,
) -> io::Result<()> {
    let target_pgid = pgid.unwrap_or(pid);
    if !is_process_group_alive(target_pgid)? {
        return Ok(());
    }
    if !policy.graceful_timeout.is_zero() {
        let _ = signal_process_group_term(pid, pgid);
        let wait_limit = policy.graceful_timeout.min(Duration::from_secs(2));
        if tokio::time::timeout(wait_limit, wait_for_process_group_exit(target_pgid))
            .await
            .is_ok()
        {
            return Ok(());
        }
    }
    if policy.escalate_to_sigkill {
        *escalated_to_sigkill = true;
        let _ = signal_process_group_kill(pid, pgid);
    }
    wait_for_process_group_exit(target_pgid).await
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
