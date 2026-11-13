// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Terminal input normalization and bounded wheel batching.

mod decoder;
mod wheel;

pub(crate) use decoder::{BracketedPaste, InputDecoder, MouseMotion, TerminalInput};
pub(crate) use wheel::WheelAccumulator;
