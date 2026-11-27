// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#![cfg(feature = "ui-grpc")]

#[path = "ui_grpc/support.rs"]
mod support;
#[path = "ui_grpc/bridge_tests.rs"]
mod bridge_tests;
#[path = "ui_grpc/host_binary_tests.rs"]
mod host_binary_tests;
