// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Versioned wire types and client bindings. The `grpc` feature adds service bindings.

tonic::include_proto!("marlin.app.v1");

/// Descriptor set generated with the bindings from the same Protobuf source.
pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("marlin-app-v1");
