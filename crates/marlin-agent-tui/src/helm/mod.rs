// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Authentic Emacs-inspired Modern Helm state machine backed by a sub-millisecond
//! resident trigram workspace search index (`WorkspaceSearchIndex`).

mod overlay;
mod types;
mod view;

pub(crate) use self::overlay::HelmOverlay;
pub(crate) use self::types::{HelmAction, HelmCandidate, HelmDisplayRow, SharedSearchIndex};
#[cfg(test)]
use self::types::{HelmParsedInput, HelmSourceKind};
pub(crate) use self::view::{helm_window_area, project_helm_ui_element};

#[cfg(test)]
#[path = "../../tests/unit/helm_source.rs"]
mod tests;
