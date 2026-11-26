// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Thin, domain-agnostic rooted-vector C-ABI codecs for Scheme-emitted 4-primitive
//! atomic effects, raw receipts, and `FleetAuditCapsuleProjection`.
//!
//! All high-order orchestration lives exclusively in Scheme (`poo-flow` /
//! `marlin-agent-core/scheme/graph-engine/`). This module only translates rooted
//! Gambit heap vectors across the FFI boundary.

use std::fmt;

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, ObjectKind,
    RootedSchemeObject, SchemeScalar,
};
use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayRequest, system_gateway_message, user_gateway_message,
};

/// Linked C-ABI symbol for single-actor driver step.
pub const MARLIN_GRAPH_ENGINE_STEP_DRIVER_SYMBOL: &str = "marlin_graph_engine_step_driver_vector";

/// Linked C-ABI symbol for multi-actor Fleet step.
pub const MARLIN_GRAPH_ENGINE_STEP_FLEET_SYMBOL: &str = "marlin_graph_engine_step_fleet_vector";

/// Linked C-ABI symbol opening a live `FleetWorkbenchController.`.
pub const MARLIN_GRAPH_ENGINE_FLEET_CONTROLLER_OPEN_SYMBOL: &str =
    "marlin_graph_engine_fleet_controller_open_vector";

/// Linked C-ABI symbol dispatching a declarative TUI `scheme-action` on `FleetWorkbenchController.`.
pub const MARLIN_GRAPH_ENGINE_FLEET_CONTROLLER_DISPATCH_SYMBOL: &str =
    "marlin_graph_engine_fleet_controller_dispatch_vector";

/// Linked C-ABI symbol projecting a non-authorizing `FleetAuditCapsule.` vector.
pub const MARLIN_GRAPH_ENGINE_FLEET_CONTROLLER_AUDIT_CAPSULE_SYMBOL: &str =
    "marlin_graph_engine_fleet_controller_audit_capsule_vector";

/// Error decoding or encoding rooted Gambit vectors at the atomic-effect C-ABI boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtomicEffectAbiError(pub String);

impl fmt::Display for AtomicEffectAbiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for AtomicEffectAbiError {}

/// Wire-decoded 4-primitive atomic effect emitted by Scheme.
#[derive(Clone, Debug, PartialEq)]
pub enum SchemeAtomicEffect {
    LlmComplete {
        effect_id: String,
        request: ModelGatewayRequest,
    },
    CasGet {
        effect_id: String,
        key: String,
    },
    CasPut {
        effect_id: String,
        key: String,
        bytes: Vec<u8>,
    },
    SandboxRun {
        effect_id: String,
        program: String,
        args: Vec<String>,
        cwd: Option<String>,
    },
}

/// Raw substrate receipt encoded back into a rooted Gambit vector for Scheme.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemeAtomicReceipt {
    LlmCompleted {
        effect_id: String,
        content: String,
    },
    CasHit {
        effect_id: String,
        bytes: Vec<u8>,
    },
    CasMiss(String),
    CasStored(String),
    SandboxExited {
        effect_id: String,
        exit_code: i32,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    },
    EffectError {
        effect_id: String,
        message: String,
    },
}

/// Decoded non-authorizing `FleetAuditCapsule.` summary projected from Scheme.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAuditCapsuleProjection {
    pub active_actor_count: usize,
    pub archived_actor_count: usize,
    pub peek_count: usize,
    pub total_active_defeaters: usize,
    pub total_human_inbox_items: usize,
    pub favored_path_collapsed: bool,
    pub authenticates_evidence: bool,
    pub authorizes_effect: bool,
}

fn read_sym(obj: RootedSchemeObject<'_>) -> Result<String, AtomicEffectAbiError> {
    let kind = obj
        .as_ref()
        .kind()
        .map_err(|e| AtomicEffectAbiError(format!("read symbol kind: {e}")))?;
    if kind != ObjectKind::Symbol {
        return Err(AtomicEffectAbiError(format!(
            "expected symbol, got {kind:?}"
        )));
    }
    let mut buf = IdentifierBuffer::default();
    obj.as_ref()
        .copy_identifier_names(&mut buf)
        .map_err(|e| AtomicEffectAbiError(format!("copy symbol name: {e}")))?;
    match buf.get(0) {
        Some((IdentifierKind::Symbol, name)) => Ok(name.to_owned()),
        other => Err(AtomicEffectAbiError(format!(
            "unexpected identifier entry: {other:?}"
        ))),
    }
}

fn read_str(obj: RootedSchemeObject<'_>) -> Result<String, AtomicEffectAbiError> {
    obj.into_string()
        .map_err(|e| AtomicEffectAbiError(format!("expected string: {e}")))?
        .to_string()
        .into_result()
        .map_err(|e| AtomicEffectAbiError(format!("decode utf-8 string: {e}")))
}

fn read_opt_str(obj: RootedSchemeObject<'_>) -> Result<Option<String>, AtomicEffectAbiError> {
    match obj
        .as_ref()
        .kind()
        .map_err(|e| AtomicEffectAbiError(format!("read opt string kind: {e}")))?
    {
        ObjectKind::String => Ok(Some(read_str(obj)?)),
        _ => Ok(None),
    }
}

fn read_nonneg_usize(obj: RootedSchemeObject<'_>) -> Result<usize, AtomicEffectAbiError> {
    match obj
        .as_ref()
        .scalar()
        .map_err(|e| AtomicEffectAbiError(format!("read fixnum: {e}")))?
    {
        SchemeScalar::Fixnum(n) if n >= 0 => Ok(n as usize),
        other => Err(AtomicEffectAbiError(format!(
            "expected non-negative fixnum, got {other:?}"
        ))),
    }
}

fn read_bool(obj: RootedSchemeObject<'_>) -> Result<bool, AtomicEffectAbiError> {
    match obj
        .as_ref()
        .scalar()
        .map_err(|e| AtomicEffectAbiError(format!("read boolean: {e}")))?
    {
        SchemeScalar::Boolean(b) => Ok(b),
        other => Err(AtomicEffectAbiError(format!(
            "expected boolean, got {other:?}"
        ))),
    }
}

fn vec_obj<'vm>(
    rt: &'vm GerbilRuntime,
    items: &[RootedSchemeObject<'vm>],
) -> Result<RootedSchemeObject<'vm>, AtomicEffectAbiError> {
    let refs: Vec<&RootedSchemeObject<'vm>> = items.iter().collect();
    rt.collection_from_objects(CollectionKind::Vector, &refs)
        .map(|c| c.into_object())
        .map_err(|e| AtomicEffectAbiError(format!("construct vector: {e}")))
}

fn sym<'vm>(
    rt: &'vm GerbilRuntime,
    name: &str,
) -> Result<RootedSchemeObject<'vm>, AtomicEffectAbiError> {
    rt.intern_identifier(IdentifierKind::Symbol, name)
        .map_err(|e| AtomicEffectAbiError(format!("intern symbol {name}: {e}")))
}

fn txt<'vm>(
    rt: &'vm GerbilRuntime,
    s: &str,
) -> Result<RootedSchemeObject<'vm>, AtomicEffectAbiError> {
    rt.string_from_utf8(s)
        .map(|v| v.into_object())
        .map_err(|e| AtomicEffectAbiError(format!("allocate string: {e}")))
}

fn fx<'vm>(
    rt: &'vm GerbilRuntime,
    n: isize,
) -> Result<RootedSchemeObject<'vm>, AtomicEffectAbiError> {
    rt.scalar_object(SchemeScalar::Fixnum(n))
        .map_err(|e| AtomicEffectAbiError(format!("allocate fixnum: {e}")))
}

/// Decodes one Scheme-emitted rooted atomic-effect vector into `SchemeAtomicEffect`.
pub fn decode_scheme_atomic_effect_vector(
    obj: RootedSchemeObject<'_>,
) -> Result<SchemeAtomicEffect, AtomicEffectAbiError> {
    let v = obj
        .into_collection(CollectionKind::Vector)
        .map_err(|e| AtomicEffectAbiError(format!("atomic effect must be vector: {e}")))?;
    let kind = read_sym(
        v.element(0)
            .map_err(|e| AtomicEffectAbiError(format!("missing effect kind: {e}")))?,
    )?;
    let effect_id = read_str(
        v.element(1)
            .map_err(|e| AtomicEffectAbiError(format!("missing effect id: {e}")))?,
    )?;
    match kind.as_str() {
        "cas-get" => Ok(SchemeAtomicEffect::CasGet {
            effect_id,
            key: read_str(
                v.element(2)
                    .map_err(|e| AtomicEffectAbiError(format!("missing cas-get key: {e}")))?,
            )?,
        }),
        "cas-put" => Ok(SchemeAtomicEffect::CasPut {
            effect_id,
            key: read_str(
                v.element(2)
                    .map_err(|e| AtomicEffectAbiError(format!("missing cas-put key: {e}")))?,
            )?,
            bytes: read_str(
                v.element(3)
                    .map_err(|e| AtomicEffectAbiError(format!("missing cas-put payload: {e}")))?,
            )?
            .into_bytes(),
        }),
        "llm-complete" => {
            let endpoint_raw = read_str(
                v.element(2)
                    .map_err(|e| AtomicEffectAbiError(format!("missing llm endpoint: {e}")))?,
            )?;
            let (prov, model) = endpoint_raw
                .split_once('/')
                .unwrap_or(("openai", endpoint_raw.as_str()));
            let msgs_vec = v
                .element(3)
                .map_err(|e| AtomicEffectAbiError(format!("missing llm messages: {e}")))?
                .into_collection(CollectionKind::Vector)
                .map_err(|e| AtomicEffectAbiError(format!("llm messages must be vector: {e}")))?;
            let mut messages = Vec::with_capacity(msgs_vec.len());
            for i in 0..msgs_vec.len() {
                let pair = msgs_vec
                    .element(i)
                    .map_err(|e| AtomicEffectAbiError(format!("message[{i}]: {e}")))?
                    .into_collection(CollectionKind::Vector)
                    .map_err(|e| AtomicEffectAbiError(format!("message[{i}] vector: {e}")))?;
                let role = read_sym(
                    pair.element(0)
                        .map_err(|e| AtomicEffectAbiError(format!("message[{i}].role: {e}")))?,
                )?;
                let body = read_str(
                    pair.element(1)
                        .map_err(|e| AtomicEffectAbiError(format!("message[{i}].body: {e}")))?,
                )?;
                messages.push(if role == "system" {
                    system_gateway_message(body)
                } else {
                    user_gateway_message(body)
                });
            }
            Ok(SchemeAtomicEffect::LlmComplete {
                effect_id,
                request: ModelGatewayRequest::new(ModelEndpoint::new(prov, model), messages),
            })
        }
        "sandbox-run" => {
            let program = read_str(
                v.element(2)
                    .map_err(|e| AtomicEffectAbiError(format!("missing sandbox program: {e}")))?,
            )?;
            let args_vec = v
                .element(3)
                .map_err(|e| AtomicEffectAbiError(format!("missing sandbox args: {e}")))?
                .into_collection(CollectionKind::Vector)
                .map_err(|e| AtomicEffectAbiError(format!("sandbox args must be vector: {e}")))?;
            let mut args = Vec::with_capacity(args_vec.len());
            for i in 0..args_vec.len() {
                args.push(read_str(args_vec.element(i).map_err(|e| {
                    AtomicEffectAbiError(format!("sandbox arg[{i}]: {e}"))
                })?)?);
            }
            let cwd = read_opt_str(
                v.element(4)
                    .map_err(|e| AtomicEffectAbiError(format!("missing sandbox cwd: {e}")))?,
            )?;
            Ok(SchemeAtomicEffect::SandboxRun {
                effect_id,
                program,
                args,
                cwd,
            })
        }
        other => Err(AtomicEffectAbiError(format!(
            "unsupported atomic effect kind `{other}`"
        ))),
    }
}

/// Encodes one raw substrate `SchemeAtomicReceipt` into a rooted Gambit vector for Scheme.
pub fn encode_scheme_atomic_receipt_vector<'vm>(
    rt: &'vm GerbilRuntime,
    receipt: &SchemeAtomicReceipt,
) -> Result<RootedSchemeObject<'vm>, AtomicEffectAbiError> {
    match receipt {
        SchemeAtomicReceipt::CasHit { effect_id, bytes } => vec_obj(
            rt,
            &[
                sym(rt, "cas-hit")?,
                txt(rt, effect_id)?,
                txt(rt, &String::from_utf8_lossy(bytes))?,
            ],
        ),
        SchemeAtomicReceipt::CasMiss(id) => vec_obj(rt, &[sym(rt, "cas-miss")?, txt(rt, id)?]),
        SchemeAtomicReceipt::CasStored(id) => vec_obj(rt, &[sym(rt, "cas-stored")?, txt(rt, id)?]),
        SchemeAtomicReceipt::LlmCompleted { effect_id, content } => vec_obj(
            rt,
            &[
                sym(rt, "llm-completed")?,
                txt(rt, effect_id)?,
                txt(rt, content)?,
            ],
        ),
        SchemeAtomicReceipt::SandboxExited {
            effect_id,
            exit_code,
            stdout,
            stderr,
        } => vec_obj(
            rt,
            &[
                sym(rt, "sandbox-exited")?,
                txt(rt, effect_id)?,
                fx(rt, *exit_code as isize)?,
                txt(rt, &String::from_utf8_lossy(stdout))?,
                txt(rt, &String::from_utf8_lossy(stderr))?,
            ],
        ),
        SchemeAtomicReceipt::EffectError { effect_id, message } => vec_obj(
            rt,
            &[
                sym(rt, "effect-error")?,
                txt(rt, effect_id)?,
                txt(rt, message)?,
            ],
        ),
    }
}

/// Decodes an 8-slot rooted `FleetAuditCapsule.` vector from `marlin-fleet-workbench-controller-audit-capsule-vector`.
pub fn decode_fleet_audit_capsule_vector(
    obj: RootedSchemeObject<'_>,
) -> Result<FleetAuditCapsuleProjection, AtomicEffectAbiError> {
    let v = obj
        .into_collection(CollectionKind::Vector)
        .map_err(|e| AtomicEffectAbiError(format!("audit capsule must be vector: {e}")))?;
    if v.len() != 8 {
        return Err(AtomicEffectAbiError(format!(
            "expected 8-slot audit capsule vector, got {}",
            v.len()
        )));
    }
    Ok(FleetAuditCapsuleProjection {
        active_actor_count: read_nonneg_usize(v.element(0).unwrap())?,
        archived_actor_count: read_nonneg_usize(v.element(1).unwrap())?,
        peek_count: read_nonneg_usize(v.element(2).unwrap())?,
        total_active_defeaters: read_nonneg_usize(v.element(3).unwrap())?,
        total_human_inbox_items: read_nonneg_usize(v.element(4).unwrap())?,
        favored_path_collapsed: read_bool(v.element(5).unwrap())?,
        authenticates_evidence: read_bool(v.element(6).unwrap())?,
        authorizes_effect: read_bool(v.element(7).unwrap())?,
    })
}

/// Decoded step output from one `FleetWorkbenchController.` dispatch call.
#[derive(Clone, Debug, PartialEq)]
pub struct SchemeFleetDispatchStep {
    pub disclosure_level: String,
    pub effects: Vec<SchemeAtomicEffect>,
    pub visible_tab_ids: Vec<String>,
    pub detour_modes: Vec<String>,
    pub active_defeater_counts: Vec<usize>,
    pub ui_element_count: usize,
}

enum FleetControllerRoot<'vm> {
    Initial(RootedSchemeObject<'vm>),
    Dispatched(gerbil_scheme::RootedSchemeCollection<'vm>),
}

/// Thin, zero-orchestration Rust handle over a live Scheme `FleetWorkbenchController.`
/// rooted in an embedded `GerbilRuntime`.
///
/// Marshals actions, 4-primitive atomic effects/receipts, and non-authorizing audit
/// capsules across the rooted Gambit C-ABI without re-implementing any Scheme semantics.
pub struct SchemeFleetSessionDriver<'vm> {
    runtime: &'vm GerbilRuntime,
    dispatch_proc: RootedSchemeObject<'vm>,
    audit_proc: RootedSchemeObject<'vm>,
    state: FleetControllerRoot<'vm>,
}

impl<'vm> SchemeFleetSessionDriver<'vm> {
    /// Opens a live `FleetWorkbenchController.` over `fleet` at `disclosure_level` (e.g. `"L1"`/`"L3"`).
    pub fn open(
        runtime: &'vm GerbilRuntime,
        open_proc: &RootedSchemeObject<'vm>,
        dispatch_proc: RootedSchemeObject<'vm>,
        audit_proc: RootedSchemeObject<'vm>,
        fleet: RootedSchemeObject<'vm>,
        disclosure_level: &str,
    ) -> Result<Self, AtomicEffectAbiError> {
        let open_in = vec_obj(runtime, &[fleet, sym(runtime, disclosure_level)?])?;
        let controller = runtime
            .call_rooted(open_proc, &open_in)
            .map_err(|e| AtomicEffectAbiError(format!("open fleet controller: {e}")))?;
        Ok(Self {
            runtime,
            dispatch_proc,
            audit_proc,
            state: FleetControllerRoot::Initial(controller),
        })
    }

    fn with_controller<T>(
        &self,
        f: impl FnOnce(&RootedSchemeObject<'_>) -> Result<T, AtomicEffectAbiError>,
    ) -> Result<T, AtomicEffectAbiError> {
        match &self.state {
            FleetControllerRoot::Initial(ctrl) => f(ctrl),
            FleetControllerRoot::Dispatched(out) => {
                let ctrl = out
                    .element(0)
                    .map_err(|e| AtomicEffectAbiError(format!("output[0] controller: {e}")))?;
                f(&ctrl)
            }
        }
    }

    /// Dispatches one declarative TUI `scheme-action` symbol onto the live Scheme controller
    /// and updates the rooted controller handle in place.
    pub fn dispatch(
        &mut self,
        action_sym: &str,
        task_id: Option<&str>,
        arg: Option<&str>,
        receipts: &[SchemeAtomicReceipt],
        auto_archive: bool,
    ) -> Result<SchemeFleetDispatchStep, AtomicEffectAbiError> {
        let rt = self.runtime;
        let task_obj = match task_id {
            Some(t) => txt(rt, t)?,
            None => rt
                .scalar_object(SchemeScalar::Boolean(false))
                .map_err(|e| AtomicEffectAbiError(format!("alloc false: {e}")))?,
        };
        let arg_obj = match arg {
            Some(a) => txt(rt, a)?,
            None => rt
                .scalar_object(SchemeScalar::Boolean(false))
                .map_err(|e| AtomicEffectAbiError(format!("alloc false: {e}")))?,
        };
        let mut rcpt_objs = Vec::with_capacity(receipts.len());
        for rcpt in receipts {
            rcpt_objs.push(encode_scheme_atomic_receipt_vector(rt, rcpt)?);
        }
        let rcpts_vec = vec_obj(rt, &rcpt_objs)?;
        let auto_obj = rt
            .scalar_object(SchemeScalar::Boolean(auto_archive))
            .map_err(|e| AtomicEffectAbiError(format!("alloc bool: {e}")))?;

        let action_obj = sym(rt, action_sym)?;
        let dispatch_in = self.with_controller(|ctrl| {
            rt.collection_from_objects(
                CollectionKind::Vector,
                &[
                    ctrl,
                    &action_obj,
                    &task_obj,
                    &arg_obj,
                    &rcpts_vec,
                    &auto_obj,
                ],
            )
            .map(|c| c.into_object())
            .map_err(|e| AtomicEffectAbiError(format!("construct dispatch vector: {e}")))
        })?;
        let out = rt
            .call_rooted(&self.dispatch_proc, &dispatch_in)
            .map_err(|e| AtomicEffectAbiError(format!("dispatch `{action_sym}`: {e}")))?
            .into_collection(CollectionKind::Vector)
            .map_err(|e| AtomicEffectAbiError(format!("dispatch output vector: {e}")))?;
        if out.len() != 7 {
            return Err(AtomicEffectAbiError(format!(
                "expected 7-slot controller dispatch output, got {}",
                out.len()
            )));
        }

        let step = {
            let disclosure_level = read_sym(
                out.element(1)
                    .map_err(|e| AtomicEffectAbiError(format!("output[1] disclosure: {e}")))?,
            )?;

            let effs_vec = out
                .element(2)
                .map_err(|e| AtomicEffectAbiError(format!("output[2] effects: {e}")))?
                .into_collection(CollectionKind::Vector)
                .map_err(|e| AtomicEffectAbiError(format!("effects vector: {e}")))?;
            let mut effects = Vec::with_capacity(effs_vec.len());
            for i in 0..effs_vec.len() {
                effects.push(decode_scheme_atomic_effect_vector(
                    effs_vec
                        .element(i)
                        .map_err(|e| AtomicEffectAbiError(format!("effect[{i}]: {e}")))?,
                )?);
            }

            let vis_vec = out
                .element(3)
                .map_err(|e| AtomicEffectAbiError(format!("output[3] visible tabs: {e}")))?
                .into_collection(CollectionKind::Vector)
                .map_err(|e| AtomicEffectAbiError(format!("visible tabs vector: {e}")))?;
            let mut visible_tab_ids = Vec::with_capacity(vis_vec.len());
            for i in 0..vis_vec.len() {
                visible_tab_ids.push(read_str(
                    vis_vec
                        .element(i)
                        .map_err(|e| AtomicEffectAbiError(format!("visible_tab[{i}]: {e}")))?,
                )?);
            }

            let detours_vec = out
                .element(4)
                .map_err(|e| AtomicEffectAbiError(format!("output[4] detours: {e}")))?
                .into_collection(CollectionKind::Vector)
                .map_err(|e| AtomicEffectAbiError(format!("detours vector: {e}")))?;
            let mut detour_modes = Vec::with_capacity(detours_vec.len());
            for i in 0..detours_vec.len() {
                detour_modes.push(read_sym(
                    detours_vec
                        .element(i)
                        .map_err(|e| AtomicEffectAbiError(format!("detour[{i}]: {e}")))?,
                )?);
            }

            let defs_vec = out
                .element(5)
                .map_err(|e| AtomicEffectAbiError(format!("output[5] defeaters: {e}")))?
                .into_collection(CollectionKind::Vector)
                .map_err(|e| AtomicEffectAbiError(format!("defeaters vector: {e}")))?;
            let mut active_defeater_counts = Vec::with_capacity(defs_vec.len());
            for i in 0..defs_vec.len() {
                active_defeater_counts.push(read_nonneg_usize(
                    defs_vec
                        .element(i)
                        .map_err(|e| AtomicEffectAbiError(format!("defeater[{i}]: {e}")))?,
                )?);
            }

            let ui_element_count = out
                .element(6)
                .map_err(|e| AtomicEffectAbiError(format!("output[6] ui elements: {e}")))?
                .into_collection(CollectionKind::Vector)
                .map_err(|e| AtomicEffectAbiError(format!("ui elements vector: {e}")))?
                .len();

            SchemeFleetDispatchStep {
                disclosure_level,
                effects,
                visible_tab_ids,
                detour_modes,
                active_defeater_counts,
                ui_element_count,
            }
        };

        self.state = FleetControllerRoot::Dispatched(out);
        Ok(step)
    }

    /// Projects the current non-authorizing `FleetAuditCapsuleProjection` from Scheme.
    pub fn audit_capsule(&self) -> Result<FleetAuditCapsuleProjection, AtomicEffectAbiError> {
        let out = self.with_controller(|ctrl| {
            self.runtime
                .call_rooted(&self.audit_proc, ctrl)
                .map_err(|e| AtomicEffectAbiError(format!("audit capsule: {e}")))
        })?;
        decode_fleet_audit_capsule_vector(out)
    }
}
