// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Loopback HTTP-JSON client connecting Marlin's [`WorkspaceSearchIndex`](agent_semantic_search::WorkspaceSearchIndex)
//! and Gerbil Scheme Search DSL to ASP's language-agnostic forwarding gateway.

mod domain_types;
mod protocol_types;
mod sync;
mod transport;

pub use domain_types::{
    ASP_FORWARDING_URL_ENV, AspForwardingEndpoint, AspLanguageId, AspProviderId,
    GeneralOntologyKindName, HttpEndpointPath, NativeAstKindName, ProviderRegistrationMode,
    ProviderRegistrationStatus, ProviderTransportProtocol, TargetedJsonSchemaDocument,
};
pub use protocol_types::{
    AspHttpHydrationReceipt, AspHttpHydrationRequest, DynamicLanguageDescriptorRequest,
    DynamicProviderCapabilities, DynamicProviderDslPackRegistration,
    DynamicProviderRegistrationAck, DynamicProviderRegistrationManifest,
    DynamicProviderRegistrationRequest, DynamicProviderSyntaxSurface, DynamicProviderTransport,
    DynamicSpecialRelationSpec, ForwardedDslPack, ForwardedKindMapping, ForwardedProviderCatalog,
    ForwardedProviderEnvelope, ForwardedProviderManifest,
};
pub use sync::{
    hydrate_workspace_pending_via_asp_http, register_linked_dynamic_language_descriptor,
    sync_linked_asp_provider_descriptors,
};
#[cfg(not(feature = "linked-ui"))]
pub use sync::{
    hydrate_workspace_from_asp_http, register_package_dynamic_language_descriptor,
    sync_package_asp_provider_descriptors,
};
pub use transport::{
    fetch_asp_provider_catalog, register_dynamic_provider_on_asp, shutdown_asp_gateway,
};
