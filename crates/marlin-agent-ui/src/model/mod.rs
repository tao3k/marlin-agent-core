// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Host-neutral UI state and transition contract.

mod commands;
mod error;
mod event;
mod frame;
mod keymap;
mod limits;
mod workbench;

pub use commands::{
    UiCommand, UiCommandArguments, UiCommandCandidates, UiCommandCapability, UiCommandName,
    UiReviewChoice, UiReviewMenu, UiToolReviewChoice, UiToolReviewDecision, UiToolReviewMenu,
};
pub use error::UiProjectionError;
pub use event::{
    UiBrowseDirection, UiBrowseRequest, UiEvent, UiIntent, UiProfileName, UiScrollDirection,
};
pub use frame::{
    UiBufferId, UiComponentId, UiContent, UiEchoPolicy, UiFrame, UiFrameRegions, UiFrameRole,
    UiFrameSpec, UiLayoutPolicy, UiRegion, UiRgb, UiScrollTarget, UiStatusItem, UiStatusTone,
    UiTheme, UiTranscriptPolicy, UiWindow,
};
pub use keymap::{
    UiCharacterSelection, UiEditorAction, UiKeyBinding, UiKeyCommand, UiKeyMode, UiKeyScope,
    UiKeyToken, UiTransient, UiTransientForeign, UiTransientHead, UiTransientId,
};
pub(crate) use limits::{
    MAX_COMMANDS, MAX_EXTENSIONS, MAX_HOST_STATUS, MAX_ID_BYTES, MAX_METHOD_FACTORIES, MAX_REGIONS,
    MAX_STATUS, MAX_TEXT_CHARS, MODELINE_SLOTS,
};
pub use workbench::{UiOpenFacts, UiWorkbench};
