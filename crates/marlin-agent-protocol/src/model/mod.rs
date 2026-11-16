// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Provider and model identity shared by the gateway and frontends.
mod endpoint;
mod identity;
pub use endpoint::{ModelEndpoint, ModelEndpointContractError};
pub use identity::{LiteLlmModelId, ModelAlias, ModelName, ModelProviderId};
