//! Explicit model choices and their provider-bound streaming gateways.

use std::{collections::BTreeMap, sync::Arc};

use marlin_agent_protocol::{
    ModelAlias, ModelEndpoint, ModelGatewayError, ModelGatewayFuture, ModelGatewayRequest,
    ModelGatewayResult,
};
use marlin_agent_stream::{ModelGatewayEventStream, ModelStreamingGateway};

pub(crate) const DEFAULT_MODEL_ALIAS: &str = "default";
pub(crate) const MAX_MODEL_CHOICES: usize = 16;

pub(crate) struct ModelRoute {
    pub(crate) alias: ModelAlias,
    pub(crate) endpoint: ModelEndpoint,
    pub(crate) gateway: Option<Arc<dyn ModelStreamingGateway>>,
}

#[derive(Clone)]
pub(crate) struct ModelCatalog {
    routes: Arc<BTreeMap<ModelAlias, ModelRoute>>,
}

impl ModelCatalog {
    pub(crate) fn new(routes: Vec<ModelRoute>) -> Result<Self, String> {
        if routes.is_empty() || routes.len() > MAX_MODEL_CHOICES {
            return Err(format!(
                "configure between 1 and {MAX_MODEL_CHOICES} models"
            ));
        }
        let mut entries = BTreeMap::new();
        for route in routes {
            let alias = route.alias.as_str();
            if alias.is_empty()
                || alias.len() > 64
                || !alias
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            {
                return Err(
                    "model aliases must be 1–64 ASCII letters, digits, hyphens or underscores"
                        .into(),
                );
            }
            route
                .endpoint
                .validate_contract()
                .map_err(|error| error.to_string())?;
            let expected = (alias != DEFAULT_MODEL_ALIAS).then_some(&route.alias);
            if route.endpoint.alias.as_ref() != expected {
                return Err("model alias does not match its endpoint".into());
            }
            if entries.insert(route.alias.clone(), route).is_some() {
                return Err("duplicate model alias".into());
            }
        }
        if entries
            .get(&ModelAlias::new(DEFAULT_MODEL_ALIAS))
            .is_none_or(|route| route.gateway.is_none())
        {
            return Err("default model is unavailable".into());
        }
        Ok(Self {
            routes: Arc::new(entries),
        })
    }

    pub(crate) fn select(&self, alias: &ModelAlias) -> Result<ModelEndpoint, &'static str> {
        let route = self.routes.get(alias).ok_or("unknown model alias")?;
        if route.gateway.is_none() {
            return Err("model credential was unavailable at startup");
        }
        Ok(route.endpoint.clone())
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = (&ModelAlias, &ModelEndpoint, bool)> {
        self.routes
            .values()
            .map(|route| (&route.alias, &route.endpoint, route.gateway.is_some()))
    }
}

impl ModelStreamingGateway for ModelCatalog {
    fn stream(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayEventStream>> {
        let alias = request
            .endpoint()
            .alias
            .clone()
            .unwrap_or_else(|| ModelAlias::new(DEFAULT_MODEL_ALIAS));
        let route = self
            .routes
            .get(&alias)
            .filter(|route| &route.endpoint == request.endpoint());
        match route.and_then(|route| route.gateway.as_ref()) {
            Some(gateway) => gateway.stream(request),
            None => Box::pin(async {
                Err(ModelGatewayError::RequestNotDispatched(
                    "model route unavailable or mismatched".into(),
                ))
            }),
        }
    }
}
