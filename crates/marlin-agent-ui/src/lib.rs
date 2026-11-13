// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! UI entry points for local Scheme projection and external gRPC clients.
//!
//! With `scheme-ui`, the caller owns VM initialization and package selection.
//! Rust frontends can disable default features and enable `grpc` for the
//! generated App Server client and schema.

#[cfg(feature = "scheme-ui")]
mod model;
#[cfg(feature = "scheme-ui")]
mod projection;
#[cfg(feature = "scheme-ui")]
mod runtime;

/// The generated Marlin client and versioned Session schema.
#[cfg(feature = "grpc")]
pub mod grpc {
    pub use marlin_agent_app_server::v1;
}

#[cfg(feature = "scheme-ui")]
pub use model::{
    UiBrowseDirection, UiBrowseRequest, UiBufferId, UiCommand, UiCommandArguments,
    UiCommandCapability, UiCommandName, UiComponentId, UiContent, UiEditorAction, UiEvent, UiFrame,
    UiFrameSpec, UiHydra, UiHydraForeign, UiHydraHead, UiIntent, UiKeyBinding, UiKeyCommand,
    UiKeyToken, UiLayoutPolicy, UiOpenFacts, UiProfileName, UiProjectionError, UiRegion,
    UiReviewChoice, UiReviewMenu, UiRgb, UiScrollDirection, UiStatusItem, UiStatusTone, UiTheme,
    UiTranscriptPolicy, UiWindow, UiWorkbench,
};
#[cfg(feature = "scheme-ui")]
pub use projection::{decode_intent, decode_review_menu, decode_workbench};
#[cfg(feature = "scheme-ui")]
pub use runtime::{
    UiLinkedExports, UiModule, UiNativeSession, dispatch_event, open_session, prepare_open_request,
    project_workbench,
};
