//! Production model-route admission through the owner-local Deck Actor pool.

use std::{num::NonZeroU16, sync::Arc, time::Duration};

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, LinkedGerbilProgram, LinkedObjectExport,
    NativeError as SchemeVmError, RootedSchemeObject, SchemeScalar,
};
use marlin_agent_protocol::{
    ModelRouteAdmissionRequest, ModelRouteAdmissionResponse, ModelRouteDecision, ModelRouteRule,
};
use marlin_agent_runtime::{
    CompiledModelRouteResolver, ModelRouteCompileError, ModelRouteSelectionProjectionError,
};
use marlin_gerbil_scheme::{
    GerbilDeckRuntimeModelRoutePolicy, GerbilDeckRuntimeModelRoutePolicyRequest,
};
use tokio::{
    sync::{mpsc, oneshot},
    time::Instant,
};
use tokio_util::sync::CancellationToken;

use crate::policy::DeckRouteCandidates;

const MAX_ACTOR_BATCH: usize = 256;

unsafe extern "C" {
    fn ___LNK_marlin__deck__actor(
        state: *mut gerbil_scheme_sys::GerbilGlobalState,
    ) -> *mut gerbil_scheme_sys::GerbilModuleOrLink;
    fn marlin_deck_route_open(root: i64) -> i64;
    fn marlin_deck_route_evaluate(root: i64) -> i64;
    fn marlin_deck_route_close(root: i64) -> i64;
}

/// Fixed VM actor count and Tokio queue bound for route admission.
#[derive(Clone, Copy, Debug)]
pub struct DeckRouteAdmissionConfig {
    pub queue_capacity: NonZeroU16,
    pub actors: NonZeroU16,
}

/// Route catalog or VM startup failure before serving requests.
#[derive(Debug, thiserror::Error)]
pub enum DeckRouteAdmissionStartupError {
    #[error(transparent)]
    Rules(#[from] ModelRouteCompileError),
    #[error("Deck requires at most 64 actors per VM")]
    ActorCount,
    #[error("cannot start Deck VM owner: {0}")]
    Owner(#[from] std::io::Error),
    #[error("Deck VM startup failed: {0}")]
    Scheme(#[from] SchemeVmError),
    #[error("Deck VM owner exited during startup")]
    OwnerExited,
}

/// Terminal Rust receipt for a call that did not admit a model route.
#[derive(Debug, thiserror::Error)]
pub enum DeckRouteAdmissionRejection {
    #[error("Deck route admission queue is full")]
    Backpressure,
    #[error("Deck route admission was cancelled")]
    Cancelled,
    #[error("Deck route admission exceeded its latency budget")]
    Deadline,
    #[error("Deck VM owner is unavailable")]
    Unavailable,
    #[error("Deck found no matching model route")]
    NoMatchingRoute,
    #[error("Deck Scheme call failed: {0}")]
    Scheme(#[from] SchemeVmError),
    #[error("Deck returned an invalid typed projection")]
    InvalidProjection,
    #[error(transparent)]
    Projection(#[from] ModelRouteSelectionProjectionError),
}

type Outcome = Result<ModelRouteAdmissionResponse, DeckRouteAdmissionRejection>;

struct Job {
    request: ModelRouteAdmissionRequest,
    cancellation: CancellationToken,
    deadline: Option<Instant>,
    reply: oneshot::Sender<Outcome>,
}

impl Job {
    fn check_active(&self) -> Result<(), DeckRouteAdmissionRejection> {
        if self.cancellation.is_cancelled() || self.reply.is_closed() {
            Err(DeckRouteAdmissionRejection::Cancelled)
        } else if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            Err(DeckRouteAdmissionRejection::Deadline)
        } else {
            Ok(())
        }
    }
}

/// Shared Tokio admission handle. The last handle closes admission and the
/// owner drains accepted calls before closing the Scheme actors.
#[derive(Clone)]
pub struct DeckRouteAdmission {
    queue: mpsc::Sender<Job>,
    resolver: Arc<CompiledModelRouteResolver>,
}

impl DeckRouteAdmission {
    /// Start and verify the resident VM before accepting route requests.
    pub fn start(
        rules: Vec<ModelRouteRule>,
        config: DeckRouteAdmissionConfig,
    ) -> Result<Self, DeckRouteAdmissionStartupError> {
        if config.actors.get() > 64 {
            return Err(DeckRouteAdmissionStartupError::ActorCount);
        }
        let resolver = Arc::new(CompiledModelRouteResolver::new(rules.clone())?);
        let policy = DeckRouteCandidates::new(rules);
        let (queue, mut jobs) = mpsc::channel::<Job>(usize::from(config.queue_capacity.get()));
        let (ready, started) = std::sync::mpsc::sync_channel(1);
        let worker_resolver = Arc::clone(&resolver);
        std::thread::Builder::new()
            .name("marlin-deck-vm".into())
            .spawn(move || {
                // SAFETY: build.rs links one compiler-owned bridge graph and its
                // returning Gambit program entry into this executable.
                let program =
                    unsafe { LinkedGerbilProgram::from_linker(___LNK_marlin__deck__actor) };
                let runtime = match GerbilRuntime::initialize_program_single_processor(program) {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = ready.send(Err(error));
                        return;
                    }
                };
                let vm = match DeckVm::open(&runtime, config.actors) {
                    Ok(vm) => vm,
                    Err(error) => {
                        let _ = ready.send(Err(error));
                        return;
                    }
                };
                let _ = ready.send(Ok(()));
                while let Some(job) = jobs.blocking_recv() {
                    let outcome = resolve(&vm, &policy, &worker_resolver, &job);
                    let _ = job.reply.send(outcome);
                }
            })?;
        started
            .recv()
            .map_err(|_| DeckRouteAdmissionStartupError::OwnerExited)??;
        Ok(Self { queue, resolver })
    }

    pub fn resolver(&self) -> &CompiledModelRouteResolver {
        &self.resolver
    }

    /// Submit an owned request. Cancellation or deadline expiry can retire
    /// the caller while an active read-only Scheme batch releases its roots.
    pub async fn admit(
        &self,
        request: ModelRouteAdmissionRequest,
        cancellation: CancellationToken,
    ) -> Outcome {
        let budget = Duration::from_millis(request.latency_budget_ms);
        let deadline = Instant::now().checked_add(budget);
        let (reply, result) = oneshot::channel();
        let job = Job {
            request,
            cancellation: cancellation.clone(),
            deadline,
            reply,
        };
        job.check_active()?;
        self.queue.try_send(job).map_err(|error| match error {
            mpsc::error::TrySendError::Full(_) => DeckRouteAdmissionRejection::Backpressure,
            mpsc::error::TrySendError::Closed(_) => DeckRouteAdmissionRejection::Unavailable,
        })?;
        tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(DeckRouteAdmissionRejection::Cancelled),
            () = tokio::time::sleep(budget) => Err(DeckRouteAdmissionRejection::Deadline),
            receipt = result => receipt.map_err(|_| DeckRouteAdmissionRejection::Unavailable)?,
        }
    }
}

fn resolve(
    vm: &DeckVm<'_>,
    policies: &DeckRouteCandidates,
    resolver: &CompiledModelRouteResolver,
    job: &Job,
) -> Outcome {
    job.check_active()?;
    let request = &job.request.route_request;
    let projection =
        GerbilDeckRuntimeModelRoutePolicyRequest::from_model_route_request([], request);
    let mut batch = Vec::new();
    let evaluate = |batch: &mut Vec<_>| {
        let decision = evaluate_batch(vm, resolver, job, &projection, batch)?;
        batch.clear();
        Ok::<_, DeckRouteAdmissionRejection>(decision)
    };
    let decision = policies.try_for_each_candidate(request, |index| {
        let original = policies
            .original_index(index)
            .ok_or(DeckRouteAdmissionRejection::InvalidProjection)?;
        let projected = policies
            .project_candidate(index, &projection.agent_scope)
            .ok_or(DeckRouteAdmissionRejection::InvalidProjection)?;
        batch.push((original, projected));
        if batch.len() == MAX_ACTOR_BATCH {
            evaluate(&mut batch)
        } else {
            Ok(None)
        }
    })?;
    let decision = match decision {
        None if !batch.is_empty() => evaluate(&mut batch)?,
        decision => decision,
    };
    decision
        .map(|decision| ModelRouteAdmissionResponse::deterministic(job.request.intent(), decision))
        .ok_or(DeckRouteAdmissionRejection::NoMatchingRoute)
}

fn evaluate_batch(
    vm: &DeckVm<'_>,
    resolver: &CompiledModelRouteResolver,
    job: &Job,
    projection: &GerbilDeckRuntimeModelRoutePolicyRequest,
    batch: &[(usize, GerbilDeckRuntimeModelRoutePolicy)],
) -> Result<Option<ModelRouteDecision>, DeckRouteAdmissionRejection> {
    job.check_active()?;
    let matched = vm.evaluate(projection, batch)?;
    job.check_active()?;
    for ((index, _), matched) in batch.iter().zip(matched) {
        if matched {
            match resolver.resolve_selected_policy_index(&job.request.route_request, *index) {
                Ok(decision) => return Ok(Some(decision)),
                Err(ModelRouteSelectionProjectionError::SelectedRuleDidNotMatch { .. }) => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(None)
}

struct DeckVm<'vm> {
    runtime: &'vm GerbilRuntime,
    evaluate: LinkedObjectExport<'vm>,
    close: LinkedObjectExport<'vm>,
    control: RootedSchemeObject<'vm>,
}

impl<'vm> DeckVm<'vm> {
    fn open(runtime: &'vm GerbilRuntime, actors: NonZeroU16) -> Result<Self, SchemeVmError> {
        // SAFETY: all three process-lived exports are in this VM's linked graph
        // and use the SDK's catch-and-project rooted entry contract.
        let (open, evaluate, close) = unsafe {
            (
                runtime.bind_object_export(marlin_deck_route_open)?,
                runtime.bind_object_export(marlin_deck_route_evaluate)?,
                runtime.bind_object_export(marlin_deck_route_close)?,
            )
        };
        let control = runtime.scalar_object(SchemeScalar::Fixnum(actors.get() as isize))?;
        open.call(&control).into_result()?;
        Ok(Self {
            runtime,
            evaluate,
            close,
            control,
        })
    }

    fn evaluate(
        &self,
        request: &GerbilDeckRuntimeModelRoutePolicyRequest,
        batch: &[(usize, GerbilDeckRuntimeModelRoutePolicy)],
    ) -> Result<Vec<bool>, DeckRouteAdmissionRejection> {
        let command = self.text(&request.command)?;
        let scope = self.text(&request.agent_scope)?;
        let policies = batch
            .iter()
            .map(|(_, policy)| {
                self.vector(&[
                    self.text(&policy.name)?,
                    self.text(&policy.provider)?,
                    self.text(&policy.model)?,
                    self.vector(&self.texts(&policy.command_prefixes)?)?,
                    self.vector(&self.texts(&policy.agent_scopes)?)?,
                    self.text(policy.context_mode.as_str())?,
                    self.text(policy.isolation_mode.as_str())?,
                ])
            })
            .collect::<Result<Vec<_>, _>>()?;
        let input = self.vector(&[command, scope, self.vector(&policies)?])?;
        let output = self
            .evaluate
            .call(&input)
            .into_result()?
            .into_collection(CollectionKind::Vector)?;
        if output.len() != batch.len() {
            return Err(DeckRouteAdmissionRejection::InvalidProjection);
        }
        (0..batch.len())
            .map(|index| match output.element(index)?.as_ref().scalar()? {
                SchemeScalar::Boolean(matched) => Ok(matched),
                _ => Err(DeckRouteAdmissionRejection::InvalidProjection),
            })
            .collect()
    }

    fn text(&self, value: &str) -> Result<RootedSchemeObject<'vm>, SchemeVmError> {
        self.runtime
            .string_from_utf8(value)
            .map(|value| value.into_object())
    }

    fn texts(&self, values: &[String]) -> Result<Vec<RootedSchemeObject<'vm>>, SchemeVmError> {
        values.iter().map(|value| self.text(value)).collect()
    }

    fn vector(
        &self,
        objects: &[RootedSchemeObject<'vm>],
    ) -> Result<RootedSchemeObject<'vm>, SchemeVmError> {
        let refs = objects.iter().collect::<Vec<_>>();
        self.runtime
            .collection_from_objects(CollectionKind::Vector, &refs)
            .map(|value| value.into_object())
    }
}

impl Drop for DeckVm<'_> {
    fn drop(&mut self) {
        let _ = self.close.call(&self.control).into_result();
    }
}

#[cfg(test)]
#[path = "../tests/unit/route_admission_queue.rs"]
mod queue_tests;
