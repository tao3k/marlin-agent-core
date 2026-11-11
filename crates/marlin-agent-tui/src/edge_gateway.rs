// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Runtime edge policy for opening a model stream.

use std::sync::Arc;

use marlin_agent_protocol::{
    ModelGatewayError, ModelGatewayFuture, ModelGatewayRequest, ModelGatewayResult,
};
use marlin_agent_runtime::{
    RuntimeEdgePolicy, RuntimeEdgePolicyError, RuntimeEdgePolicyReceipt, RuntimeEdgeService,
};
use marlin_agent_stream::{ModelGatewayEventStream, ModelStreamingGateway};
use tokio::sync::Mutex;
use tower::{BoxError, Service, ServiceExt, service_fn};

#[derive(Clone)]
pub(crate) struct SharedGateway(Arc<dyn ModelStreamingGateway>);

impl SharedGateway {
    pub(crate) fn new(gateway: impl ModelStreamingGateway) -> Self {
        Self(Arc::new(gateway))
    }
}

impl ModelStreamingGateway for SharedGateway {
    fn stream(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        self.0.stream(request)
    }
}

/// Applies the runtime's service-edge policy to opening the stream only.
/// The policy does not hold a concurrency permit over stream consumption and
/// its timeout does not cover subsequent stream items.
#[derive(Clone)]
pub(crate) struct RuntimeStreamGateway {
    service: Arc<Mutex<RuntimeEdgeService<ModelGatewayRequest, ModelGatewayEventStream>>>,
    receipt: RuntimeEdgePolicyReceipt,
}

impl RuntimeStreamGateway {
    pub(crate) fn new<G: ModelStreamingGateway>(
        gateway: G,
        policy: RuntimeEdgePolicy,
    ) -> Result<Self, RuntimeEdgePolicyError> {
        let gateway = Arc::new(gateway);
        let service = service_fn(move |request| {
            let gateway = Arc::clone(&gateway);
            async move {
                gateway
                    .stream(request)
                    .await
                    .map_err(|error| -> BoxError { Box::new(error) })
            }
        });
        let (service, receipt) = policy.apply(service)?;
        Ok(Self {
            service: Arc::new(Mutex::new(service)),
            receipt,
        })
    }

    pub(crate) fn receipt(&self) -> &RuntimeEdgePolicyReceipt {
        &self.receipt
    }
}

impl ModelStreamingGateway for RuntimeStreamGateway {
    fn stream(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        let service = Arc::clone(&self.service);
        Box::pin(async move {
            let future = {
                let mut service = service.lock().await;
                service
                    .ready()
                    .await
                    .map_err(|error| {
                        ModelGatewayError::RequestNotDispatched(format!(
                            "runtime stream-open edge was not ready: {error}"
                        ))
                    })?
                    .call(request)
            };
            future.await.map_err(edge_error)
        })
    }
}

fn edge_error(error: BoxError) -> ModelGatewayError {
    match error.downcast::<ModelGatewayError>() {
        Ok(original) => *original,
        Err(error) if error.is::<tower::timeout::error::Elapsed>() => {
            ModelGatewayError::StreamOpenTimeout
        }
        Err(error) => ModelGatewayError::Completion(format!(
            "runtime stream-open edge rejected request: {error}"
        )),
    }
}
