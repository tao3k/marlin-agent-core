//! Runtime-owned Tower service policy and receipts.

use std::{error::Error, fmt, time::Duration};

use serde::{Deserialize, Serialize};
use tower::{
    BoxError, Service, ServiceBuilder, ServiceExt, limit::ConcurrencyLimitLayer,
    load_shed::LoadShedLayer, timeout::TimeoutLayer, util::BoxService,
};

/// Boxed runtime edge service produced after applying a resilience policy.
pub type RuntimeEdgeService<Request, Response> = BoxService<Request, Response, BoxError>;

/// Tower layer applied to a runtime service edge.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeEdgeLayer {
    ConcurrencyLimit,
    LoadShed,
    Timeout,
}

impl RuntimeEdgeLayer {
    /// Stable layer identifier used in receipts.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConcurrencyLimit => "concurrency-limit",
            Self::LoadShed => "load-shed",
            Self::Timeout => "timeout",
        }
    }
}

/// Runtime service resilience policy.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEdgePolicy {
    timeout_ms: Option<u64>,
    concurrency_limit: Option<usize>,
    load_shed: bool,
}

impl RuntimeEdgePolicy {
    /// Creates a policy with no resilience layers enabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a timeout layer in milliseconds.
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Adds a concurrency limit layer.
    pub fn with_concurrency_limit(mut self, concurrency_limit: usize) -> Self {
        self.concurrency_limit = Some(concurrency_limit);
        self
    }

    /// Enables or disables load shedding.
    pub fn with_load_shed(mut self, load_shed: bool) -> Self {
        self.load_shed = load_shed;
        self
    }

    /// Configured timeout in milliseconds.
    pub fn timeout_ms(&self) -> Option<u64> {
        self.timeout_ms
    }

    /// Configured concurrency limit.
    pub fn concurrency_limit(&self) -> Option<usize> {
        self.concurrency_limit
    }

    /// Whether load shedding is enabled.
    pub fn load_shed(&self) -> bool {
        self.load_shed
    }

    /// Builds a typed receipt for this policy.
    pub fn receipt(&self) -> Result<RuntimeEdgePolicyReceipt, RuntimeEdgePolicyError> {
        self.validate()?;
        Ok(RuntimeEdgePolicyReceipt::from_policy(self))
    }

    /// Applies this policy to a Tower service and returns the boxed service plus receipt.
    pub fn apply<Request, S>(
        &self,
        service: S,
    ) -> Result<
        (
            RuntimeEdgeService<Request, S::Response>,
            RuntimeEdgePolicyReceipt,
        ),
        RuntimeEdgePolicyError,
    >
    where
        Request: Send + 'static,
        S: Service<Request> + Send + 'static,
        S::Error: Into<BoxError> + Send + Sync + 'static,
        S::Future: Send + 'static,
        S::Response: Send + 'static,
    {
        let receipt = self.receipt()?;
        let mut service = service
            .map_err(|error| -> BoxError { error.into() })
            .boxed();

        if let Some(concurrency_limit) = self.concurrency_limit {
            service = ServiceBuilder::new()
                .layer(ConcurrencyLimitLayer::new(concurrency_limit))
                .service(service)
                .boxed();
        }

        if self.load_shed {
            service = ServiceBuilder::new()
                .layer(LoadShedLayer::new())
                .service(service)
                .boxed();
        }

        if let Some(timeout_ms) = self.timeout_ms {
            service = ServiceBuilder::new()
                .layer(TimeoutLayer::new(Duration::from_millis(timeout_ms)))
                .service(service)
                .boxed();
        }

        Ok((service, receipt))
    }

    fn validate(&self) -> Result<(), RuntimeEdgePolicyError> {
        if self.timeout_ms == Some(0) {
            return Err(RuntimeEdgePolicyError::ZeroTimeout);
        }
        if self.concurrency_limit == Some(0) {
            return Err(RuntimeEdgePolicyError::ZeroConcurrencyLimit);
        }
        Ok(())
    }
}

/// Receipt describing the Tower layers attached to a runtime edge.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEdgePolicyReceipt {
    timeout_ms: Option<u64>,
    concurrency_limit: Option<usize>,
    load_shed: bool,
    layers: Vec<RuntimeEdgeLayer>,
}

impl RuntimeEdgePolicyReceipt {
    fn from_policy(policy: &RuntimeEdgePolicy) -> Self {
        let mut layers = Vec::new();
        if policy.concurrency_limit.is_some() {
            layers.push(RuntimeEdgeLayer::ConcurrencyLimit);
        }
        if policy.load_shed {
            layers.push(RuntimeEdgeLayer::LoadShed);
        }
        if policy.timeout_ms.is_some() {
            layers.push(RuntimeEdgeLayer::Timeout);
        }

        Self {
            timeout_ms: policy.timeout_ms,
            concurrency_limit: policy.concurrency_limit,
            load_shed: policy.load_shed,
            layers,
        }
    }

    /// Configured timeout in milliseconds.
    pub fn timeout_ms(&self) -> Option<u64> {
        self.timeout_ms
    }

    /// Configured concurrency limit.
    pub fn concurrency_limit(&self) -> Option<usize> {
        self.concurrency_limit
    }

    /// Whether load shedding is enabled.
    pub fn load_shed(&self) -> bool {
        self.load_shed
    }

    /// Layers attached to the runtime edge in wrapping order.
    pub fn layers(&self) -> &[RuntimeEdgeLayer] {
        self.layers.as_slice()
    }
}

/// Invalid runtime edge policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeEdgePolicyError {
    ZeroTimeout,
    ZeroConcurrencyLimit,
}

impl fmt::Display for RuntimeEdgePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroTimeout => {
                write!(formatter, "runtime edge timeout must be greater than zero")
            }
            Self::ZeroConcurrencyLimit => write!(
                formatter,
                "runtime edge concurrency limit must be greater than zero"
            ),
        }
    }
}

impl Error for RuntimeEdgePolicyError {}
