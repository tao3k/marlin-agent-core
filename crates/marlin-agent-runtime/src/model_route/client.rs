//! Runtime-facing model gateway protocol re-exports.

use std::fmt;
use std::sync::Arc;

pub use marlin_agent_protocol::{
    ModelGateway, ModelGatewayCompletionChoice, ModelGatewayCompletionOptions,
    ModelGatewayCompletionResponse, ModelGatewayError, ModelGatewayFuture, ModelGatewayMessage,
    ModelGatewayMessageRole, ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport,
    assistant_gateway_message, system_gateway_message, user_gateway_message,
};
use tokio::sync::Mutex;
use tower::{BoxError, Service, ServiceExt, service_fn};

use crate::resilience::{
    RuntimeEdgePolicy, RuntimeEdgePolicyError, RuntimeEdgePolicyReceipt, RuntimeEdgeService,
};

/// Runtime-owned resilience wrapper for provider-neutral model gateways.
///
/// The wrapper keeps the public gateway contract provider-neutral while applying
/// Tower timeout, load-shed, and concurrency layers at the runtime edge.
pub struct RuntimeEdgeModelGateway {
    service: Arc<Mutex<RuntimeEdgeService<ModelGatewayRequest, ModelGatewayCompletionResponse>>>,
    edge_policy_receipt: RuntimeEdgePolicyReceipt,
}

impl fmt::Debug for RuntimeEdgeModelGateway {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeEdgeModelGateway")
            .field("edge_policy_receipt", &self.edge_policy_receipt)
            .finish_non_exhaustive()
    }
}

impl RuntimeEdgeModelGateway {
    /// Applies a runtime edge policy to a provider-neutral model gateway.
    pub fn new<G>(gateway: G, policy: RuntimeEdgePolicy) -> Result<Self, RuntimeEdgePolicyError>
    where
        G: ModelGateway + Send + Sync + 'static,
    {
        let gateway = Arc::new(gateway);
        let service = service_fn(move |request: ModelGatewayRequest| {
            let gateway = Arc::clone(&gateway);
            async move {
                gateway
                    .complete(request)
                    .await
                    .map_err(|error| -> BoxError { Box::new(error) })
            }
        });
        let (service, edge_policy_receipt) = policy.apply(service)?;

        Ok(Self {
            service: Arc::new(Mutex::new(service)),
            edge_policy_receipt,
        })
    }

    /// Receipt describing the runtime edge layers applied to the gateway.
    pub fn edge_policy_receipt(&self) -> &RuntimeEdgePolicyReceipt {
        &self.edge_policy_receipt
    }
}

impl ModelGateway for RuntimeEdgeModelGateway {
    fn complete(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayCompletionResponse>> {
        let service = Arc::clone(&self.service);
        Box::pin(async move {
            let future = {
                let mut service = service.lock().await;
                service
                    .ready()
                    .await
                    .map_err(runtime_edge_model_gateway_error)?
                    .call(request)
            };

            future.await.map_err(runtime_edge_model_gateway_error)
        })
    }
}

fn runtime_edge_model_gateway_error(error: BoxError) -> ModelGatewayError {
    ModelGatewayError::Completion(format!(
        "runtime edge policy rejected model gateway request: {error}"
    ))
}
