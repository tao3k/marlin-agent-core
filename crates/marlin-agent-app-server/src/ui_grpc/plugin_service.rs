// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::collections::BTreeSet;

use marlin_agent_runtime::plugin::{
    CapabilityId, ContributionId, PanelActivationId, PanelPluginHost, PanelProjectId, PluginDenial,
    PluginId, PublisherId, ReleaseAttestation as RuntimeAttestation, ScopedPanelActionRequest,
    SignedPanelRelease, VerifiedPanelRelease,
};
use tokio::sync::broadcast;
use tonic::{Request, Response, Status};

use crate::wire::marlin::{
    common::v1::ReceiptRef,
    plugin::v1::{
        DisableWorkflowRequest, DisableWorkflowResponse, EnableWorkflowRequest,
        EnableWorkflowResponse, GetWorkflowEvidenceRequest, GetWorkflowEvidenceResponse,
        InstallSignedReleaseRequest, InstallSignedReleaseResponse, InstalledReleaseSummary,
        ListReleasesRequest, ListReleasesResponse, OpenScopedPanelStreamRequest,
        OpenScopedPanelStreamResponse, PanelAssetDescriptor, ProjectGrantReceipt,
        ScopedPanelActionReceipt, ScopedPanelEvent, SetProjectGrantsRequest,
        SetProjectGrantsResponse, SubmitScopedPanelActionRequest, SubmitScopedPanelActionResponse,
        WorkflowActivationReceipt, WorkflowDisableReceipt, plugin_service_server::PluginService,
    },
};

use super::{
    host::MyGoBridgeHost,
    state::{BoxStream, map_denial, mint_receipt, now_unix_ms, to_proto_cardinality, to_proto_runtime},
};

fn summarize_verified_panel(
    panel_host: &PanelPluginHost,
    verified: &VerifiedPanelRelease,
) -> InstalledReleaseSummary {
    let status = panel_host.active_generation_for(verified.id());
    let enabled = status.enabled;
    let active_generation = status.active_generation;
    let panel_assets: Vec<PanelAssetDescriptor> = verified
        .contributions()
        .iter()
        .map(|(cid, c)| PanelAssetDescriptor {
            contribution_id: cid.as_str().to_owned(),
            runtime: to_proto_runtime(c.panel.runtime),
            slot_id: c.panel.slot_id.as_str().to_owned(),
            cardinality: to_proto_cardinality(c.panel.cardinality),
            slot_key: c.panel.slot_key.clone(),
            entry_path: c.panel.entry_path.clone().unwrap_or_default(),
            archive_sha256: verified.digest().as_bytes().to_vec(),
            byte_size: 0,
            read_scopes: c.panel.read_scopes.clone(),
            declared_actions: c.panel.declared_actions.clone(),
        })
        .collect();
    InstalledReleaseSummary {
        plugin_id: verified.id().as_str().to_owned(),
        version: format!("{:?}", verified.version()),
        publisher_id: verified.attestation().publisher.as_str().to_owned(),
        bundle_digest_sha256: verified.digest().as_bytes().to_vec(),
        enabled,
        active_generation,
        declared_capabilities: Vec::new(),
        granted_capabilities: Vec::new(),
        panel_assets,
    }
}

#[tonic::async_trait]
impl PluginService for MyGoBridgeHost {
    type OpenScopedPanelStreamStream = BoxStream<OpenScopedPanelStreamResponse>;

    async fn list_releases(
        &self,
        request: Request<ListReleasesRequest>,
    ) -> Result<Response<ListReleasesResponse>, Status> {
        self.check_auth(&request)?;
        let st = self.lock_state()?;
        Ok(Response::new(ListReleasesResponse {
            releases: st
                .panel_host
                .installed_releases()
                .map(|v| summarize_verified_panel(&st.panel_host, v))
                .collect(),
        }))
    }

    async fn install_signed_release(
        &self,
        request: Request<InstallSignedReleaseRequest>,
    ) -> Result<Response<InstallSignedReleaseResponse>, Status> {
        self.check_auth(&request)?;
        self.ensure_governance_live("install_signed_release")?;
        let Some(env) = request.into_inner().release else {
            return Ok(Response::new(InstallSignedReleaseResponse {
                installed: false,
                summary: None,
                denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
            }));
        };
        let Some(att_pb) = env.attestation else {
            return Ok(Response::new(InstallSignedReleaseResponse {
                installed: false,
                summary: None,
                denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
            }));
        };
        let Ok(publisher) = PublisherId::try_from(att_pb.publisher_id) else {
            return Ok(Response::new(InstallSignedReleaseResponse {
                installed: false,
                summary: None,
                denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
            }));
        };
        let Ok(source_revision): Result<[u8; 20], _> = att_pb.source_revision.try_into() else {
            return Ok(Response::new(InstallSignedReleaseResponse {
                installed: false,
                summary: None,
                denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
            }));
        };
        let Ok(build_evidence_digest): Result<[u8; 32], _> =
            att_pb.build_evidence_digest.try_into()
        else {
            return Ok(Response::new(InstallSignedReleaseResponse {
                installed: false,
                summary: None,
                denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
            }));
        };
        let Ok(sig_arr): Result<[u8; 64], _> = env.ed25519_signature.try_into() else {
            return Ok(Response::new(InstallSignedReleaseResponse {
                installed: false,
                summary: None,
                denial: Some(map_denial(PluginDenial::InvalidSignature)),
            }));
        };

        let mut st = self.lock_state()?;
        let installed_id = match st.panel_host.install(SignedPanelRelease {
            manifest: &env.manifest_toml,
            attestation: RuntimeAttestation {
                publisher,
                source_revision,
                build_evidence_digest,
            },
            signature: &sig_arr,
        }) {
            Ok(verified) => verified.id().clone(),
            Err(denial) => {
                return Ok(Response::new(InstallSignedReleaseResponse {
                    installed: false,
                    summary: None,
                    denial: Some(map_denial(denial)),
                }));
            }
        };
        let verified = st
            .panel_host
            .get_release(&installed_id)
            .expect("just installed release must exist");
        let mut summary = summarize_verified_panel(&st.panel_host, verified);
        for asset in &mut summary.panel_assets {
            asset.byte_size = env.manifest_toml.len() as u64;
        }
        Ok(Response::new(InstallSignedReleaseResponse {
            installed: true,
            summary: Some(summary),
            denial: None,
        }))
    }

    async fn enable_workflow(
        &self,
        request: Request<EnableWorkflowRequest>,
    ) -> Result<Response<EnableWorkflowResponse>, Status> {
        self.check_auth(&request)?;
        self.ensure_governance_live("enable_workflow")?;
        let req = request.into_inner();
        let Ok(pid) = PluginId::try_from(req.plugin_id.clone()) else {
            return Ok(Response::new(EnableWorkflowResponse {
                receipt: Some(WorkflowActivationReceipt {
                    activated: false,
                    activation_id: String::new(),
                    generation: 0,
                    effective_settings_sha256: Vec::new(),
                    receipt: None,
                    denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
                }),
            }));
        };
        let Ok(cid) = ContributionId::try_from(req.contribution_id.clone()) else {
            return Ok(Response::new(EnableWorkflowResponse {
                receipt: Some(WorkflowActivationReceipt {
                    activated: false,
                    activation_id: String::new(),
                    generation: 0,
                    effective_settings_sha256: Vec::new(),
                    receipt: None,
                    denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
                }),
            }));
        };

        let mut st = self.lock_state()?;
        let act = match st.panel_host.enable_panel(
            PanelProjectId::new(req.project_id),
            &pid,
            &cid,
            &req.settings_toml,
        ) {
            Ok(rec) => rec,
            Err(denial) => {
                return Ok(Response::new(EnableWorkflowResponse {
                    receipt: Some(WorkflowActivationReceipt {
                        activated: false,
                        activation_id: String::new(),
                        generation: 0,
                        effective_settings_sha256: Vec::new(),
                        receipt: None,
                        denial: Some(map_denial(denial)),
                    }),
                }));
            }
        };
        let first_scope = act
            .read_scopes
            .iter()
            .find(|t| t.as_str() != "host.secrets")
            .cloned();
        let rref = mint_receipt(&mut st, "plugin-activate", act.activation_id.as_bytes());
        let event_seq = st.receipt_seq;
        drop(st);
        if let Some(topic) = first_scope {
            let _ = self.panel_tx.send(ScopedPanelEvent {
                event_id: format!("ev-activate-{event_seq}"),
                plugin_id: req.plugin_id,
                activation_generation: act.generation,
                topic,
                payload_json: br#"{"nodeId":"verify_receipts","status":"activated"}"#.to_vec(),
            });
        }
        Ok(Response::new(EnableWorkflowResponse {
            receipt: Some(WorkflowActivationReceipt {
                activated: true,
                activation_id: act.activation_id,
                generation: act.generation,
                effective_settings_sha256: act.settings_digest.to_vec(),
                receipt: Some(rref),
                denial: None,
            }),
        }))
    }

    async fn disable_workflow(
        &self,
        request: Request<DisableWorkflowRequest>,
    ) -> Result<Response<DisableWorkflowResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let mut st = self.lock_state()?;
        let activation_id = PanelActivationId::new(req.activation_id);
        match st.panel_host.disable_panel(&activation_id) {
            Ok(closed) => {
                let rref = ReceiptRef {
                    receipt_id: format!("rcpt-plugin-disable-{}", closed.closed_generation),
                    owner_domain: "marlin-agent-runtime".to_owned(),
                    digest_sha256: closed.closure_digest.to_vec(),
                    committed_at_unix_ms: now_unix_ms(),
                };
                Ok(Response::new(DisableWorkflowResponse {
                    receipt: Some(WorkflowDisableReceipt {
                        disabled: true,
                        closed_generation: closed.closed_generation,
                        receipt: Some(rref),
                        denial: None,
                    }),
                }))
            }
            Err(denial) => Ok(Response::new(DisableWorkflowResponse {
                receipt: Some(WorkflowDisableReceipt {
                    disabled: false,
                    closed_generation: 0,
                    receipt: None,
                    denial: Some(map_denial(denial)),
                }),
            })),
        }
    }

    async fn set_project_grants(
        &self,
        request: Request<SetProjectGrantsRequest>,
    ) -> Result<Response<SetProjectGrantsResponse>, Status> {
        self.check_auth(&request)?;
        self.ensure_governance_live("set_project_grants")?;
        let req = request.into_inner();
        let Ok(pid) = PluginId::try_from(req.plugin_id.clone()) else {
            return Ok(Response::new(SetProjectGrantsResponse {
                receipt: Some(ProjectGrantReceipt {
                    updated: false,
                    granted_capabilities: Vec::new(),
                    receipt: None,
                    denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
                }),
            }));
        };
        let mut caps = Vec::with_capacity(req.granted_capabilities.len());
        for raw in req.granted_capabilities {
            let Ok(cap) = CapabilityId::try_from(raw) else {
                return Ok(Response::new(SetProjectGrantsResponse {
                    receipt: Some(ProjectGrantReceipt {
                        updated: false,
                        granted_capabilities: Vec::new(),
                        receipt: None,
                        denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
                    }),
                }));
            };
            caps.push(cap);
        }
        let mut st = self.lock_state()?;
        match st
            .panel_host
            .set_project_grants(PanelProjectId::new(req.project_id), pid, caps)
        {
            Ok(granted) => {
                let rref = mint_receipt(&mut st, "plugin-grant", req.plugin_id.as_bytes());
                Ok(Response::new(SetProjectGrantsResponse {
                    receipt: Some(ProjectGrantReceipt {
                        updated: true,
                        granted_capabilities: granted
                            .into_iter()
                            .map(|c| c.as_str().to_owned())
                            .collect(),
                        receipt: Some(rref),
                        denial: None,
                    }),
                }))
            }
            Err(denial) => Ok(Response::new(SetProjectGrantsResponse {
                receipt: Some(ProjectGrantReceipt {
                    updated: false,
                    granted_capabilities: Vec::new(),
                    receipt: None,
                    denial: Some(map_denial(denial)),
                }),
            })),
        }
    }

    async fn get_workflow_evidence(
        &self,
        request: Request<GetWorkflowEvidenceRequest>,
    ) -> Result<Response<GetWorkflowEvidenceResponse>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let st = self.lock_state()?;
        let activation_id = PanelActivationId::new(req.activation_id);
        let Some(ev) = st.panel_host.evidence(&activation_id) else {
            return Err(Status::not_found("activation evidence not found"));
        };
        let effect_receipts: Vec<ReceiptRef> = ev
            .actions
            .iter()
            .map(|a| ReceiptRef {
                receipt_id: a.receipt_id.clone(),
                owner_domain: "marlin-agent-runtime".to_owned(),
                digest_sha256: a.receipt_digest.to_vec(),
                committed_at_unix_ms: now_unix_ms(),
            })
            .collect();
        Ok(Response::new(GetWorkflowEvidenceResponse {
            activation_id: ev.activation_id.clone(),
            bundle_digest_sha256: ev.bundle_digest.as_bytes().to_vec(),
            project_digest_sha256: ev.settings_digest.to_vec(),
            effect_receipts,
            replay_bundle: None,
        }))
    }

    async fn open_scoped_panel_stream(
        &self,
        request: Request<OpenScopedPanelStreamRequest>,
    ) -> Result<Response<Self::OpenScopedPanelStreamStream>, Status> {
        self.check_auth(&request)?;
        let req = request.into_inner();
        let pid = PluginId::try_from(req.plugin_id.clone())
            .map_err(|_| Status::invalid_argument("invalid plugin_id"))?;
        let cid = ContributionId::try_from(req.contribution_id.clone())
            .map_err(|_| Status::invalid_argument("invalid contribution_id"))?;
        let allowed_topics: BTreeSet<String> = {
            let st = self.lock_state()?;
            let project_id = PanelProjectId::new(req.project_id);
            st.panel_host
                .active_read_scopes(&project_id, &pid, &cid, req.activation_generation)
                .map_err(|_| Status::failed_precondition("plugin generation not active"))?
        };
        let mut rx = self.panel_tx.subscribe();
        let plugin_id = req.plugin_id;
        let active_gen = req.activation_generation;
        let stream = async_stream::try_stream! {
            loop {
                match rx.recv().await {
                    Ok(ev) => {
                        if ev.plugin_id == plugin_id
                            && ev.activation_generation == active_gen
                            && ev.topic != "host.secrets"
                            && allowed_topics.contains(&ev.topic)
                        {
                            yield OpenScopedPanelStreamResponse { event: Some(ev) };
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn submit_scoped_panel_action(
        &self,
        request: Request<SubmitScopedPanelActionRequest>,
    ) -> Result<Response<SubmitScopedPanelActionResponse>, Status> {
        self.check_auth(&request)?;
        self.ensure_governance_live("submit_scoped_panel_action")?;
        let req = request.into_inner();
        let Ok(pid) = PluginId::try_from(req.plugin_id.clone()) else {
            return Ok(Response::new(SubmitScopedPanelActionResponse {
                receipt: Some(ScopedPanelActionReceipt {
                    admitted: false,
                    result_json: None,
                    receipt: None,
                    denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
                }),
            }));
        };
        let Ok(cid) = ContributionId::try_from(req.contribution_id.clone()) else {
            return Ok(Response::new(SubmitScopedPanelActionResponse {
                receipt: Some(ScopedPanelActionReceipt {
                    admitted: false,
                    result_json: None,
                    receipt: None,
                    denial: Some(map_denial(PluginDenial::InvalidDeclaration)),
                }),
            }));
        };
        let mut st = self.lock_state()?;
        let project_id = PanelProjectId::new(req.project_id);
        match st.panel_host.admit_scoped_action(ScopedPanelActionRequest {
            project_id: &project_id,
            plugin_id: &pid,
            contribution_id: &cid,
            generation: req.activation_generation,
            action_name: &req.action_name,
            args_json: &req.args_json,
            idempotency_key: &req.idempotency_key,
        }) {
            Ok(action_rec) => Ok(Response::new(SubmitScopedPanelActionResponse {
                receipt: Some(ScopedPanelActionReceipt {
                    admitted: true,
                    result_json: Some(br#"{"status":"admitted"}"#.to_vec()),
                    receipt: Some(ReceiptRef {
                        receipt_id: action_rec.receipt_id,
                        owner_domain: "marlin-agent-runtime".to_owned(),
                        digest_sha256: action_rec.receipt_digest.to_vec(),
                        committed_at_unix_ms: now_unix_ms(),
                    }),
                    denial: None,
                }),
            })),
            Err(denial) => Ok(Response::new(SubmitScopedPanelActionResponse {
                receipt: Some(ScopedPanelActionReceipt {
                    admitted: false,
                    result_json: None,
                    receipt: None,
                    denial: Some(map_denial(denial)),
                }),
            })),
        }
    }
}
