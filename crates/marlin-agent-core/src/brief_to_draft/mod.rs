// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Trusted Brief-to-Draft settings, execution and replay.

#[cfg(unix)]
mod execution;
mod registration;
#[cfg(unix)]
mod replay;

#[cfg(unix)]
pub use execution::{BriefRun, BriefRunError, run};
pub use registration::registration;
#[cfg(unix)]
pub use replay::BriefRunEvidence;
