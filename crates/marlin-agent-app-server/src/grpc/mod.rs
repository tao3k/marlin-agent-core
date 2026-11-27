// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! gRPC `AppServer` service implementation and transport helper boundaries.

mod helpers;
mod service;

pub(crate) use helpers::{
    MAX_MESSAGES, MAX_REQUEST_BYTES, MAX_TURN_TIME, account_output, check_session_deadline,
    session_until,
};
pub use service::AppServer;
