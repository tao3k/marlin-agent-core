//! Native Deck runtime backed model-route resolver.

use marlin_agent_protocol::{ModelRouteDecision, ModelRouteRequest, ModelRouteRule};
use marlin_agent_runtime::{CompiledModelRouteResolver, ModelRouteCompileError};
use marlin_gerbil_scheme::GerbilDeckRuntimeNativeModelRouteSelector;

use crate::{error::DeckRuntimeNativeRouteError, policy::DeckRuntimeNativePolicySet};

/// Resolves model routes through the linked native Deck selector, then projects
/// the selected policy index through the Rust runtime resolver.
#[derive(Clone)]
pub struct DeckRuntimeNativeRouteResolver {
    selector: GerbilDeckRuntimeNativeModelRouteSelector,
    policy_set: DeckRuntimeNativePolicySet,
    runtime_resolver: CompiledModelRouteResolver,
}

impl DeckRuntimeNativeRouteResolver {
    /// Builds a native-backed resolver from a selector and route rules.
    pub fn new(
        selector: GerbilDeckRuntimeNativeModelRouteSelector,
        rules: Vec<ModelRouteRule>,
    ) -> Result<Self, ModelRouteCompileError> {
        let policy_set = DeckRuntimeNativePolicySet::new(rules);
        let runtime_resolver = CompiledModelRouteResolver::new(policy_set.rules().to_vec())?;

        Ok(Self {
            selector,
            policy_set,
            runtime_resolver,
        })
    }

    /// Resolves a route through the native selector.
    pub fn resolve(
        &self,
        request: &ModelRouteRequest,
    ) -> Result<Option<ModelRouteDecision>, DeckRuntimeNativeRouteError> {
        let native_request = self.policy_set.native_request_for(request);
        let Some(native_policy_index) = self.selector.select_policy_index(&native_request)? else {
            return Ok(None);
        };
        let Some(original_policy_index) = self
            .policy_set
            .original_index_for_native(native_policy_index)
        else {
            return Err(DeckRuntimeNativeRouteError::UnknownNativePolicyIndex {
                native_policy_index,
                policies_len: self.policy_set.native_policies_len(),
            });
        };

        self.runtime_resolver
            .resolve_selected_policy_index(request, original_policy_index)
            .map(Some)
            .map_err(DeckRuntimeNativeRouteError::from)
    }

    /// Returns the Rust resolver used for final decision projection.
    pub fn runtime_resolver(&self) -> &CompiledModelRouteResolver {
        &self.runtime_resolver
    }
}
