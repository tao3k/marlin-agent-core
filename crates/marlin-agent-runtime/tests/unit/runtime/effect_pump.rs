// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use marlin_agent_protocol::{ModelEndpoint, ModelGatewayRequest, user_gateway_message};
use marlin_agent_runtime::{
    AtomicEffect, AtomicEffectExecutor, AtomicEffectReceipt, CasGetEffectPayload,
    CasHitReceiptPayload, CasPutEffectPayload, LlmCompleteEffectPayload,
    LlmCompletedReceiptPayload, SandboxExitedReceiptPayload, SandboxRunEffectPayload,
    run_atomic_effect_batch,
};

struct StubSubstrate {
    cas: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl AtomicEffectExecutor for StubSubstrate {
    fn execute_atomic<'a>(
        &'a self,
        effect: AtomicEffect,
    ) -> Pin<Box<dyn Future<Output = AtomicEffectReceipt> + Send + 'a>> {
        Box::pin(async move {
            match effect {
                AtomicEffect::LlmComplete(payload) => {
                    let prompt = payload
                        .request
                        .messages()
                        .last()
                        .map(|m| m.content.clone())
                        .unwrap_or_default();
                    AtomicEffectReceipt::LlmCompleted(LlmCompletedReceiptPayload {
                        effect_id: payload.effect_id,
                        content: format!("echo:{prompt}"),
                    })
                }
                AtomicEffect::CasGet(payload) => {
                    if let Some(bytes) = self
                        .cas
                        .lock()
                        .expect("cas lock poisoned")
                        .get(&payload.key)
                        .cloned()
                    {
                        AtomicEffectReceipt::CasHit(CasHitReceiptPayload {
                            effect_id: payload.effect_id,
                            bytes,
                        })
                    } else {
                        AtomicEffectReceipt::CasMiss(payload.effect_id)
                    }
                }
                AtomicEffect::CasPut(payload) => {
                    self.cas
                        .lock()
                        .expect("cas lock poisoned")
                        .insert(payload.key, payload.bytes);
                    AtomicEffectReceipt::CasStored(payload.effect_id)
                }
                AtomicEffect::SandboxRun(payload) => {
                    AtomicEffectReceipt::SandboxExited(SandboxExitedReceiptPayload {
                        effect_id: payload.effect_id,
                        exit_code: 0,
                        stdout: format!("ran:{}", payload.program).into_bytes(),
                        stderr: Vec::new(),
                    })
                }
            }
        })
    }
}

#[tokio::test]
async fn atomic_effect_batch_executes_four_primitives_and_preserves_order() {
    let substrate = Arc::new(StubSubstrate {
        cas: Mutex::new(BTreeMap::from([(
            "k-hit".to_owned(),
            b"cached-payload".to_vec(),
        )])),
    });
    let req = ModelGatewayRequest::new(
        ModelEndpoint::new("openai", "gpt-test"),
        vec![user_gateway_message("ping")],
    );

    let receipts = run_atomic_effect_batch(
        substrate,
        vec![
            AtomicEffect::CasGet(CasGetEffectPayload {
                effect_id: "e1".to_owned(),
                key: "k-hit".to_owned(),
            }),
            AtomicEffect::LlmComplete(LlmCompleteEffectPayload {
                effect_id: "e2".to_owned(),
                request: req,
            }),
            AtomicEffect::CasPut(CasPutEffectPayload {
                effect_id: "e3".to_owned(),
                key: "k-new".to_owned(),
                bytes: b"v2".to_vec(),
            }),
            AtomicEffect::SandboxRun(SandboxRunEffectPayload {
                effect_id: "e4".to_owned(),
                program: "true".to_owned(),
                args: Vec::new(),
                cwd: None,
            }),
        ],
    )
    .await;

    assert_eq!(
        receipts,
        vec![
            AtomicEffectReceipt::CasHit(CasHitReceiptPayload {
                effect_id: "e1".to_owned(),
                bytes: b"cached-payload".to_vec(),
            }),
            AtomicEffectReceipt::LlmCompleted(LlmCompletedReceiptPayload {
                effect_id: "e2".to_owned(),
                content: "echo:ping".to_owned(),
            }),
            AtomicEffectReceipt::CasStored("e3".to_owned()),
            AtomicEffectReceipt::SandboxExited(SandboxExitedReceiptPayload {
                effect_id: "e4".to_owned(),
                exit_code: 0,
                stdout: b"ran:true".to_vec(),
                stderr: Vec::new(),
            }),
        ]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sm19_atomic_effect_batch_bounds_peak_concurrency_and_preserves_id_on_panic() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    struct ConcurrencyAndPanicSubstrate {
        in_flight: AtomicUsize,
        peak_in_flight: AtomicUsize,
    }

    impl AtomicEffectExecutor for ConcurrencyAndPanicSubstrate {
        fn execute_atomic<'a>(
            &'a self,
            effect: AtomicEffect,
        ) -> Pin<Box<dyn Future<Output = AtomicEffectReceipt> + Send + 'a>> {
            Box::pin(async move {
                let current = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                self.peak_in_flight.fetch_max(current, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(25)).await;
                self.in_flight.fetch_sub(1, Ordering::SeqCst);
                let id = effect.effect_id().to_owned();
                if id == "sha256:panic-target-7" {
                    panic!("simulated worker panic for {id}");
                }
                AtomicEffectReceipt::CasMiss(id)
            })
        }
    }

    let substrate = Arc::new(ConcurrencyAndPanicSubstrate {
        in_flight: AtomicUsize::new(0),
        peak_in_flight: AtomicUsize::new(0),
    });
    let effects: Vec<AtomicEffect> = (0..48)
        .map(|i| {
            let effect_id = if i == 7 {
                "sha256:panic-target-7".to_owned()
            } else {
                format!("sha256:batch-{i}")
            };
            AtomicEffect::CasGet(CasGetEffectPayload {
                effect_id,
                key: format!("k-{i}"),
            })
        })
        .collect();

    let receipts = run_atomic_effect_batch(Arc::clone(&substrate), effects).await;
    let peak = substrate.peak_in_flight.load(Ordering::SeqCst);
    assert!(
        peak <= 16,
        "unbounded JoinSet fan-out reached {peak} concurrent workers (expected <= 16)"
    );
    assert_eq!(receipts.len(), 48);
    assert_eq!(
        receipts[7].effect_id(),
        "sha256:panic-target-7",
        "panicked worker lost caller effect_id correlation: got {:?}",
        receipts[7]
    );
}
