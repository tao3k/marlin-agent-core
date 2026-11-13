// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed projection of the selected Scheme UI workbench through the SDK.
//!
//! The caller owns VM initialization and startup package selection. The SDK's
//! standard module loader resolves the selected publication and procedures.

mod model;
mod projection;
mod runtime;

pub use model::{
    UiBrowseDirection, UiBrowseRequest, UiBufferId, UiCommand, UiCommandArguments,
    UiCommandCapability, UiCommandName, UiComponentId, UiContent, UiEditorAction, UiEvent, UiFrame,
    UiFrameSpec, UiHydra, UiHydraForeign, UiHydraHead, UiIntent, UiKeyBinding, UiKeyCommand,
    UiKeyToken, UiLayoutPolicy, UiOpenFacts, UiProfileName, UiProjectionError, UiRegion,
    UiReviewChoice, UiReviewMenu, UiRgb, UiScrollDirection, UiStatusItem, UiStatusTone, UiTheme,
    UiTranscriptPolicy, UiWindow, UiWorkbench,
};
pub use projection::{decode_intent, decode_review_menu, decode_workbench};
pub use runtime::{
    UiLinkedExports, UiModule, UiNativeSession, dispatch_event, open_session, prepare_open_request,
    project_workbench,
};
