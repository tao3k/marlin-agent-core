// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed requests and presentation intents for the UI dispatcher.

use super::{UiBufferId, UiCommandName, UiComponentId, UiProjectionError};

/// Input events accepted by the Scheme UI dispatcher.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiEvent {
    NextFocus,
    PreviousFocus,
    SelectFocus(UiComponentId),
    Submit,
    Dismiss,
    ScrollUp,
    ScrollDown,
    SelectProfile(UiProfileName),
    SelectBuffer(UiBufferId),
    SelectListRow {
        list: UiComponentId,
        row: UiComponentId,
    },
    /// A successfully admitted command, for Scheme-owned completion history.
    RecordCommand(UiCommandName),
    /// A generic Scheme-owned UI state action that re-projects `UiElement`s cleanly
    /// without custom Rust widget bypasses.
    SchemeAction {
        action: UiCommandName,
        argument: Option<String>,
    },
}

/// A bounded Scheme profile identity; Rust does not enumerate profile names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiProfileName(String);

impl UiProfileName {
    pub fn new(value: &str) -> Result<Self, UiProjectionError> {
        if value.is_empty()
            || value.len() > 32
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(UiProjectionError::Invalid("invalid UI profile name"));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Direction for moving through a visible collection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiBrowseDirection {
    Next,
    Previous,
}

/// One bounded Scheme-owned candidate navigation request.
#[derive(Clone, Copy, Debug)]
pub struct UiBrowseRequest<'a> {
    pub query: &'a str,
    pub previous: Option<&'a UiCommandName>,
    pub direction: UiBrowseDirection,
    pub steps: u16,
    /// Optional runtime-admitted subset, retaining Scheme's original order.
    pub allowed: Option<&'a [UiCommandName]>,
}

/// A presentation proposal; only the runtime can authorize its effects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiIntent {
    SubmitDraft,
    Focus(UiComponentId),
    Scroll(UiScrollDirection),
    SelectBuffer(UiBufferId),
}

/// Direction for scrolling a UI region.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiScrollDirection {
    Up,
    Down,
}
