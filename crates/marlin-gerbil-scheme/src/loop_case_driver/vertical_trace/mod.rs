// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Owns the vertical-trace parsing branch while keeping typed receipt data,
//! row decoding, and trace verification in acyclic leaf modules.

mod parsing;
mod row;
mod types;

pub use parsing::{
    parse_gerbil_loop_case_driver_vertical_trace, verify_gerbil_loop_case_driver_vertical_trace,
};
pub use types::{GerbilLoopCaseDriverVerticalTraceError, GerbilLoopCaseDriverVerticalTraceReceipt};
