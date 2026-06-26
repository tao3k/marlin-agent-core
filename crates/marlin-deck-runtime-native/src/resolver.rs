//! Native Deck runtime backed model-route resolver.

use marlin_agent_protocol::{ModelRouteDecision, ModelRouteRequest, ModelRouteRule};
use marlin_agent_runtime::{
    CompiledModelRouteResolver, ModelRouteCompileError, ModelRouteSelectionProjectionError,
};
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
        self.policy_set.try_for_each_native_policy_candidate(
            request,
            |candidate_native_policy_index| {
                let Some(native_request) = self
                    .policy_set
                    .native_request_for_candidate(request, candidate_native_policy_index)
                else {
                    return Err(DeckRuntimeNativeRouteError::UnknownNativePolicyIndex {
                        native_policy_index: candidate_native_policy_index,
                        policies_len: self.policy_set.native_policies_len(),
                    });
                };
                let Some(candidate_request_policy_index) =
                    self.selector.select_policy_index(&native_request)?
                else {
                    return Ok(None);
                };
                if candidate_request_policy_index != 0 {
                    return Err(DeckRuntimeNativeRouteError::UnknownNativePolicyIndex {
                        native_policy_index: candidate_request_policy_index,
                        policies_len: 1,
                    });
                }
                let native_policy_index = candidate_native_policy_index;
                let Some(original_policy_index) = self
                    .policy_set
                    .original_index_for_native(native_policy_index)
                else {
                    return Err(DeckRuntimeNativeRouteError::UnknownNativePolicyIndex {
                        native_policy_index,
                        policies_len: self.policy_set.native_policies_len(),
                    });
                };

                match self
                    .runtime_resolver
                    .resolve_selected_policy_index(request, original_policy_index)
                {
                    Ok(decision) => Ok(Some(decision)),
                    Err(ModelRouteSelectionProjectionError::SelectedRuleDidNotMatch { .. }) => {
                        Ok(None)
                    }
                    Err(error) => Err(DeckRuntimeNativeRouteError::from(error)),
                }
            },
        )
    }

    /// Returns the Rust resolver used for final decision projection.
    pub fn runtime_resolver(&self) -> &CompiledModelRouteResolver {
        &self.runtime_resolver
    }
}
