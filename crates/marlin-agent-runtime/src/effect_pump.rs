// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Thin, domain-agnostic atomic effect pump for Scheme-owned high-order orchestration.
//!
//! All high-level and high-order semantics (FunFlow purity/caching, DAG frontier
//! scheduling, Scoped Detour transitions, competing hypothesis evaluation, and
//! Two-Zone prompt assembly) live exclusively in Scheme (`poo-flow`). Rust executes
//! only the four primitive I/O effects below in parallel and returns raw receipts.

use std::fmt::Write as _;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use marlin_agent_protocol::ModelGatewayRequest;
use ring::digest::{SHA256, digest};

/// Computes a deterministic `sha256:<64-lowercase-hex>` content digest for raw CAS payloads/keys.
pub fn sha256_prefixed_hex(bytes: &[u8]) -> String {
    let hash = digest(&SHA256, bytes);
    let mut out = String::with_capacity("sha256:".len() + hash.as_ref().len() * 2);
    out.push_str("sha256:");
    for byte in hash.as_ref() {
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

/// Payload for `AtomicEffect::LlmComplete`.
#[derive(Clone, Debug, PartialEq)]
pub struct LlmCompleteEffectPayload {
    pub effect_id: String,
    pub request: ModelGatewayRequest,
}

/// Payload for `AtomicEffect::CasGet`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CasGetEffectPayload {
    pub effect_id: String,
    pub key: String,
}

/// Payload for `AtomicEffect::CasPut`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CasPutEffectPayload {
    pub effect_id: String,
    pub key: String,
    pub bytes: Vec<u8>,
}

/// Payload for `AtomicEffect::SandboxRun`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxRunEffectPayload {
    pub effect_id: String,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
}

/// Primitive I/O effect emitted by Scheme orchestration for Rust runtime execution.
#[derive(Clone, Debug, PartialEq)]
pub enum AtomicEffect {
    /// Execute a single raw model gateway request with Scheme-assembled messages.
    LlmComplete(LlmCompleteEffectPayload),
    /// Read raw bytes from content-addressed storage by Scheme-computed key.
    CasGet(CasGetEffectPayload),
    /// Write raw bytes into content-addressed storage under a Scheme-computed key.
    CasPut(CasPutEffectPayload),
    /// Execute an isolated command inside Marlin's sandbox/process substrate.
    SandboxRun(SandboxRunEffectPayload),
}

impl AtomicEffect {
    /// Returns the correlation identifier of this atomic effect.
    pub fn effect_id(&self) -> &str {
        match self {
            Self::LlmComplete(payload) => &payload.effect_id,
            Self::CasGet(payload) => &payload.effect_id,
            Self::CasPut(payload) => &payload.effect_id,
            Self::SandboxRun(payload) => &payload.effect_id,
        }
    }
}

/// Payload for `AtomicEffectReceipt::LlmCompleted`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LlmCompletedReceiptPayload {
    pub effect_id: String,
    pub content: String,
}

/// Payload for `AtomicEffectReceipt::CasHit`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CasHitReceiptPayload {
    pub effect_id: String,
    pub bytes: Vec<u8>,
}

/// Payload for `AtomicEffectReceipt::SandboxExited`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxExitedReceiptPayload {
    pub effect_id: String,
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Payload for `AtomicEffectReceipt::EffectError`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectErrorReceiptPayload {
    pub effect_id: String,
    pub message: String,
}

/// Raw substrate receipt returned to Scheme after executing one `AtomicEffect`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtomicEffectReceipt {
    LlmCompleted(LlmCompletedReceiptPayload),
    CasHit(CasHitReceiptPayload),
    CasMiss(String),
    CasStored(String),
    SandboxExited(SandboxExitedReceiptPayload),
    EffectError(EffectErrorReceiptPayload),
}

impl AtomicEffectReceipt {
    /// Returns the correlation identifier echoed from the originating `AtomicEffect`.
    pub fn effect_id(&self) -> &str {
        match self {
            Self::LlmCompleted(payload) => &payload.effect_id,
            Self::CasHit(payload) => &payload.effect_id,
            Self::CasMiss(effect_id) | Self::CasStored(effect_id) => effect_id,
            Self::SandboxExited(payload) => &payload.effect_id,
            Self::EffectError(payload) => &payload.effect_id,
        }
    }
}

/// Leaf runtime substrate executing a single primitive `AtomicEffect`.
pub trait AtomicEffectExecutor: Send + Sync {
    fn execute_atomic<'a>(
        &'a self,
        effect: AtomicEffect,
    ) -> Pin<Box<dyn Future<Output = AtomicEffectReceipt> + Send + 'a>>;
}

/// Executes a batch of Scheme-emitted `AtomicEffect`s concurrently via Tokio `JoinSet`
/// and returns raw `AtomicEffectReceipt`s in deterministic input order.
pub async fn run_atomic_effect_batch<E>(
    executor: Arc<E>,
    effects: Vec<AtomicEffect>,
) -> Vec<AtomicEffectReceipt>
where
    E: AtomicEffectExecutor + 'static,
{
    let total = effects.len();
    let mut join_set = tokio::task::JoinSet::new();
    for (idx, effect) in effects.into_iter().enumerate() {
        let exec = Arc::clone(&executor);
        let fallback_id = effect.effect_id().to_owned();
        join_set.spawn(async move {
            let receipt = exec.execute_atomic(effect).await;
            (idx, fallback_id, receipt)
        });
    }

    let mut ordered: Vec<Option<AtomicEffectReceipt>> = vec![None; total];
    while let Some(joined) = join_set.join_next().await {
        if let Ok((idx, _id, receipt)) = joined {
            ordered[idx] = Some(receipt);
        }
    }

    ordered
        .into_iter()
        .enumerate()
        .map(|(idx, slot)| {
            slot.unwrap_or_else(|| {
                AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                    effect_id: format!("effect-{idx}"),
                    message: "atomic effect worker terminated unexpectedly".to_owned(),
                })
            })
        })
        .collect()
}
