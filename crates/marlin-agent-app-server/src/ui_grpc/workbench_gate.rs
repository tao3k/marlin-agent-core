// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{collections::HashMap, sync::Arc};

use tokio::sync::broadcast;
use tonic::{Request, Response, Status};

use crate::wire::marlin::{
    common::v1::{DenialCode, TypedDenial},
    device::v1::DeviceInvocationRequest,
    gate::v1::{
        GateDecisionReceipt, GateKind, GetGateReceiptRequest, GetGateReceiptResponse,
        ListPendingGatesRequest, ListPendingGatesResponse, PendingGate,
        SubmitGraphHumanDecisionRequest, SubmitGraphHumanDecisionResponse,
        SubmitToolReviewDecisionRequest, SubmitToolReviewDecisionResponse,
        WatchPendingGatesRequest, WatchPendingGatesResponse,
        approval_gate_service_server::ApprovalGateService,
    },
    ui::v1::{
        BrowseCommandsRequest, BrowseCommandsResponse, DispatchUiEventRequest,
        DispatchUiEventResponse, OpenWorkbenchRequest, OpenWorkbenchResponse, UiCommandCandidates,
        UiEmptyMarker, UiIntent, WatchWorkbenchRequest, WatchWorkbenchResponse,
        dispatch_ui_event_request, ui_intent,
        ui_workbench_service_server::UiWorkbenchService,
    },
};

use super::{
    host::MyGoBridgeHost,
    state::{
        BoxStream, ClaimedGraphHumanDecision, DispatchedUiCommand, build_snapshot, mint_receipt,
        profile_for_target,
    },
};

#[tonic::async_trait]
impl UiWorkbenchService for MyGoBridgeHost {
    type WatchWorkbenchStream = BoxStream<WatchWorkbenchResponse>;

    async fn open_workbench(
        &self,
        request: Request<OpenWorkbenchRequest>,
    ) -> Result<Response<OpenWorkbenchResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let mut st = self.lock_state()?;
        if !req.session_id.is_empty() {
            st.session_label = req.session_id;
        }
        let resolved_profile = req
            .preferred_profile
            .unwrap_or_else(|| profile_for_target(&st, req.client_target_kind).to_owned());
        st.profile_name = resolved_profile;
        if let Some(overlay) = st.overlays.get(&req.client_target_kind) {
            st.slots = overlay.slots.clone();
        }
        let snap = build_snapshot(&st);
        Ok(Response::new(OpenWorkbenchResponse {
            workbench: Some(snap),
        }))
    }

    async fn watch_workbench(
        &self,
        request: Request<WatchWorkbenchRequest>,
    ) -> Result<Response<Self::WatchWorkbenchStream>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let initial = {
            let st = self.lock_state()?;
            if st.generation > req.after_generation {
                Some(build_snapshot(&st))
            } else {
                None
            }
        };
        let mut rx = self.wb_tx.subscribe();
        let state = Arc::clone(&self.state);
        let mut last_sent_gen = initial
            .as_ref()
            .map_or(req.after_generation, |s| s.generation);
        let stream = async_stream::try_stream! {
            if let Some(snap) = initial {
                yield WatchWorkbenchResponse { workbench: Some(snap) };
            }
            loop {
                match rx.recv().await {
                    Ok(snap) => {
                        if snap.generation > last_sent_gen {
                            last_sent_gen = snap.generation;
                            yield WatchWorkbenchResponse { workbench: Some(snap) };
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let resync = {
                            let st = state.lock().expect("bridge mutex poisoned");
                            build_snapshot(&st)
                        };
                        if resync.generation > last_sent_gen {
                            last_sent_gen = resync.generation;
                            yield WatchWorkbenchResponse { workbench: Some(resync) };
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn dispatch_ui_event(
        &self,
        request: Request<DispatchUiEventRequest>,
    ) -> Result<Response<DispatchUiEventResponse>, Status> {
        self.check_auth(&request)?;
        self.ensure_governance_live("dispatch_ui_event")?;
        let req = request.into_inner();
        let (intent, next, dispatched_cmd) = {
            let mut st = self.lock_state()?;
            if req.expected_generation != 0 && req.expected_generation != st.generation {
                return Ok(Response::new(DispatchUiEventResponse {
                    admitted: false,
                    intent: None,
                    next_workbench: None,
                    denial: Some(TypedDenial {
                        code: DenialCode::StaleGeneration as i32,
                        message: format!(
                            "expected generation {}, current {}",
                            req.expected_generation, st.generation
                        ),
                        owner_id: Some("marlin.ui.v1".to_owned()),
                        details: HashMap::new(),
                    }),
                }));
            }

            let mut dispatched_cmd = None;
            let intent = match req.event {
                Some(dispatch_ui_event_request::Event::SelectProfile(prof)) => {
                    st.profile_name = prof;
                    st.generation += 1;
                    Some(UiIntent {
                        intent: Some(ui_intent::Intent::FocusComponentId(st.focus_id.clone())),
                    })
                }
                Some(dispatch_ui_event_request::Event::SelectFocus(target)) => {
                    st.focus_id = target.clone();
                    st.generation += 1;
                    Some(UiIntent {
                        intent: Some(ui_intent::Intent::FocusComponentId(target)),
                    })
                }
                Some(dispatch_ui_event_request::Event::SelectBuffer(buf)) => {
                    st.generation += 1;
                    Some(UiIntent {
                        intent: Some(ui_intent::Intent::SelectBufferId(buf)),
                    })
                }
                Some(dispatch_ui_event_request::Event::RecordCommand(cmd)) => {
                    if let Some(alias) = cmd.strip_prefix("model.select ") {
                        let trimmed = alias.trim();
                        if !trimmed.is_empty() {
                            st.model_label = trimmed.to_owned();
                        }
                    }
                    st.generation += 1;
                    dispatched_cmd = Some(DispatchedUiCommand {
                        session_id: req.session_id,
                        command: cmd,
                        generation: st.generation,
                    });
                    Some(UiIntent {
                        intent: Some(ui_intent::Intent::SubmitDraft(UiEmptyMarker {})),
                    })
                }
                _ => Some(UiIntent {
                    intent: Some(ui_intent::Intent::SubmitDraft(UiEmptyMarker {})),
                }),
            };
            let next = build_snapshot(&st);
            (intent, next, dispatched_cmd)
        };

        // Execute registered PackageBridge mutations (e.g. `magit.*`, future
        // Scheme/Rust package actions) under the governance gate uniformly.
        if let Some(ref cmd_ev) = dispatched_cmd {
            match self.bridges.dispatch_command(&cmd_ev.command).await {
                Ok(Some(outcome)) => {
                    if let Some(traj) = outcome.trajectory {
                        self.publish_graph_trajectory_event(traj);
                    }
                    if !outcome.ok {
                        return Ok(Response::new(DispatchUiEventResponse {
                            admitted: false,
                            intent: None,
                            next_workbench: Some(next),
                            denial: Some(TypedDenial {
                                code: DenialCode::Unspecified as i32,
                                message: outcome.summary,
                                owner_id: Some(outcome.owner_id),
                                details: HashMap::new(),
                            }),
                        }));
                    }
                }
                Ok(None) => {}
                Err((owner_id, err)) => {
                    return Ok(Response::new(DispatchUiEventResponse {
                        admitted: false,
                        intent: None,
                        next_workbench: Some(next),
                        denial: Some(TypedDenial {
                            code: DenialCode::Unspecified as i32,
                            message: err,
                            owner_id: Some(owner_id),
                            details: HashMap::new(),
                        }),
                    }));
                }
            }
        }

        let _ = self.wb_tx.send(next.clone());
        if let Some(cmd_ev) = dispatched_cmd {
            let _ = self.ui_cmd_tx.send(cmd_ev);
        }
        Ok(Response::new(DispatchUiEventResponse {
            admitted: true,
            intent,
            next_workbench: Some(next),
            denial: None,
        }))
    }

    async fn browse_commands(
        &self,
        request: Request<BrowseCommandsRequest>,
    ) -> Result<Response<BrowseCommandsResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let st = self.lock_state()?;
        let snap = build_snapshot(&st);
        let q = req.query.to_lowercase();
        let names: Vec<String> = snap
            .commands
            .into_iter()
            .map(|c| c.name)
            .filter(|n| q.is_empty() || n.to_lowercase().contains(&q))
            .collect();
        let selected = names.first().cloned();
        Ok(Response::new(BrowseCommandsResponse {
            candidates: Some(UiCommandCandidates { names, selected }),
        }))
    }
}

#[tonic::async_trait]
impl ApprovalGateService for MyGoBridgeHost {
    type WatchPendingGatesStream = BoxStream<WatchPendingGatesResponse>;

    async fn list_pending_gates(
        &self,
        request: Request<ListPendingGatesRequest>,
    ) -> Result<Response<ListPendingGatesResponse>, Status> {
        self.check_auth(&request)?;
        let st = self.lock_state()?;
        Ok(Response::new(ListPendingGatesResponse {
            gates: st.pending_gates.values().cloned().collect(),
        }))
    }

    async fn watch_pending_gates(
        &self,
        request: Request<WatchPendingGatesRequest>,
    ) -> Result<Response<Self::WatchPendingGatesStream>, Status> {
        self.check_auth(&request)?;
        let existing: Vec<PendingGate> = {
            let st = self.lock_state()?;
            st.pending_gates.values().cloned().collect()
        };
        let mut rx = self.gate_tx.subscribe();
        let state = Arc::clone(&self.state);
        let stream = async_stream::try_stream! {
            for gate in existing {
                yield WatchPendingGatesResponse { gate: Some(gate) };
            }
            loop {
                match rx.recv().await {
                    Ok(gate) => {
                        yield WatchPendingGatesResponse { gate: Some(gate) };
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let resync: Vec<PendingGate> = {
                            let st = state.lock().expect("bridge mutex poisoned");
                            st.pending_gates.values().cloned().collect()
                        };
                        for gate in resync {
                            yield WatchPendingGatesResponse { gate: Some(gate) };
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn submit_tool_review_decision(
        &self,
        request: Request<SubmitToolReviewDecisionRequest>,
    ) -> Result<Response<SubmitToolReviewDecisionResponse>, Status> {
        self.check_auth(&request)?;
        self.ensure_governance_live("submit_tool_review_decision")?;
        let req = request.into_inner();
        let mut st = self.lock_state()?;
        if st.claimed_gates.contains_key(&req.gate_id) {
            return Ok(Response::new(SubmitToolReviewDecisionResponse {
                receipt: Some(GateDecisionReceipt {
                    gate_id: req.gate_id,
                    kind: GateKind::ModelToolReview as i32,
                    claimed: false,
                    receipt: None,
                    denial: Some(TypedDenial {
                        code: DenialCode::StaleOrdinal as i32,
                        message: "one-shot gate already claimed".to_owned(),
                        owner_id: Some("marlin.gate.v1".to_owned()),
                        details: HashMap::new(),
                    }),
                }),
            }));
        }
        let Some(pending) = st.pending_gates.remove(&req.gate_id) else {
            return Ok(Response::new(SubmitToolReviewDecisionResponse {
                receipt: Some(GateDecisionReceipt {
                    gate_id: req.gate_id,
                    kind: GateKind::ModelToolReview as i32,
                    claimed: false,
                    receipt: None,
                    denial: Some(TypedDenial {
                        code: DenialCode::NotFound as i32,
                        message: "pending gate not found".to_owned(),
                        owner_id: Some("marlin.gate.v1".to_owned()),
                        details: HashMap::new(),
                    }),
                }),
            }));
        };
        let rref = mint_receipt(&mut st, "gate", req.gate_id.as_bytes());
        let receipt = GateDecisionReceipt {
            gate_id: pending.gate_id.clone(),
            kind: pending.kind,
            claimed: true,
            receipt: Some(rref.clone()),
            denial: None,
        };
        st.insert_claimed_gate(pending.gate_id.clone(), receipt.clone());
        let should_dispatch_device = pending.kind == GateKind::DeviceCapability as i32
            && req.decision
                == crate::wire::marlin::gate::v1::ToolReviewDecision::ApproveOnce as i32;
        let session_label = st.session_label.clone();
        drop(st);
        if should_dispatch_device {
            let _ = self.device_inv_tx.send(DeviceInvocationRequest {
                invocation_id: format!("inv-{}", pending.gate_id),
                session_id: session_label,
                capability_id: pending.tool_or_node_name,
                gate_receipt: Some(rref),
                args_json:
                    br#"{"title":"Marlin Approved","body":"Authorized via one-shot Rust gate"}"#
                        .to_vec(),
                timeout_ms: 5_000,
            });
        }
        Ok(Response::new(SubmitToolReviewDecisionResponse {
            receipt: Some(receipt),
        }))
    }

    async fn submit_graph_human_decision(
        &self,
        request: Request<SubmitGraphHumanDecisionRequest>,
    ) -> Result<Response<SubmitGraphHumanDecisionResponse>, Status> {
        self.check_auth(&request)?;
        self.ensure_governance_live("submit_graph_human_decision")?;
        let req = request.into_inner();
        let mut st = self.lock_state()?;
        if st.claimed_gates.contains_key(&req.gate_id) {
            return Ok(Response::new(SubmitGraphHumanDecisionResponse {
                receipt: Some(GateDecisionReceipt {
                    gate_id: req.gate_id,
                    kind: GateKind::GraphHumanGate as i32,
                    claimed: false,
                    receipt: None,
                    denial: Some(TypedDenial {
                        code: DenialCode::StaleOrdinal as i32,
                        message: "one-shot gate already claimed".to_owned(),
                        owner_id: Some("marlin.gate.v1".to_owned()),
                        details: HashMap::new(),
                    }),
                }),
            }));
        }
        let Some(pending) = st.pending_gates.remove(&req.gate_id) else {
            return Ok(Response::new(SubmitGraphHumanDecisionResponse {
                receipt: Some(GateDecisionReceipt {
                    gate_id: req.gate_id,
                    kind: GateKind::GraphHumanGate as i32,
                    claimed: false,
                    receipt: None,
                    denial: Some(TypedDenial {
                        code: DenialCode::NotFound as i32,
                        message: "pending gate not found".to_owned(),
                        owner_id: Some("marlin.gate.v1".to_owned()),
                        details: HashMap::new(),
                    }),
                }),
            }));
        };
        let rref = mint_receipt(&mut st, "graph-gate", req.gate_id.as_bytes());
        let claimed_event = ClaimedGraphHumanDecision {
            gate_id: pending.gate_id.clone(),
            decision: req.decision,
            note: req.note,
            receipt_id: rref.receipt_id.clone(),
        };
        let receipt = GateDecisionReceipt {
            gate_id: pending.gate_id.clone(),
            kind: pending.kind,
            claimed: true,
            receipt: Some(rref),
            denial: None,
        };
        st.insert_claimed_gate(pending.gate_id, receipt.clone());
        drop(st);
        let _ = self.graph_gate_tx.send(claimed_event);
        Ok(Response::new(SubmitGraphHumanDecisionResponse {
            receipt: Some(receipt),
        }))
    }

    async fn get_gate_receipt(
        &self,
        request: Request<GetGateReceiptRequest>,
    ) -> Result<Response<GetGateReceiptResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let st = self.lock_state()?;
        let receipt = st
            .claimed_gates
            .get(&req.gate_id)
            .cloned()
            .unwrap_or(GateDecisionReceipt {
                gate_id: req.gate_id,
                kind: GateKind::Unspecified as i32,
                claimed: false,
                receipt: None,
                denial: Some(TypedDenial {
                    code: DenialCode::NotFound as i32,
                    message: "gate receipt not found".to_owned(),
                    owner_id: Some("marlin.gate.v1".to_owned()),
                    details: HashMap::new(),
                }),
            });
        Ok(Response::new(GetGateReceiptResponse {
            receipt: Some(receipt),
        }))
    }
}
