// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! The optional Marlin App Server transport for external clients.

#[cfg(feature = "grpc")]
mod grpc;
#[cfg(feature = "grpc")]
pub use grpc::AppServer;
#[cfg(feature = "grpc")]
mod session;
#[cfg(feature = "grpc")]
pub use session::SessionBinding;

/// Versioned wire types and the generated client and service bindings.
#[cfg(feature = "grpc")]
pub mod v1 {
    tonic::include_proto!("marlin.app.v1");

    /// Descriptor set generated with the bindings from the same Protobuf source.
    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("marlin-app-v1");
}
