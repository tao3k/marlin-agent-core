// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Scheme command descriptors and finite human-review menu values.

use super::UiProjectionError;
use marlin_agent_protocol::HumanDecision;

/// A displayable command name; this value does not authorize a handler.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UiCommandName(String);

impl UiCommandName {
    /// Check a partial command without scanning past the admitted name limit.
    pub fn is_valid_prefix(value: &str) -> bool {
        value.len() <= 32
            && value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    }

    pub fn new(value: &str) -> Result<Self, UiProjectionError> {
        if value.is_empty() || !Self::is_valid_prefix(value) {
            return Err(UiProjectionError::Invalid("invalid command name"));
        }
        Ok(Self(value.to_owned()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A command descriptor projected from the selected Scheme UI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiCommand {
    pub name: UiCommandName,
    pub description: String,
    pub arguments: UiCommandArguments,
    pub capability: UiCommandCapability,
    pub value: Option<String>,
}

/// Ordered Scheme candidates and the user's still-visible locked selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiCommandCandidates {
    pub names: Vec<UiCommandName>,
    pub selected: Option<UiCommandName>,
}

/// Scheme-selected Minibuffer presentation for an existing runtime human gate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiReviewMenu {
    pub prompt: String,
    pub choices: Vec<UiReviewChoice>,
}

/// Presentation of one finite Rust-owned human decision variant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiReviewChoice {
    pub decision: HumanDecision,
    pub label: String,
    pub description: String,
}

/// Scheme-selected Mini Buffer choices for a committed model tool request.
/// These are presentation intents; the session journal validates the request
/// identity and commits the actual review decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiToolReviewMenu {
    pub prompt: String,
    pub choices: Vec<UiToolReviewChoice>,
    pub details_heading: String,
}

/// One visual action in the tool Ask Mini Buffer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiToolReviewChoice {
    pub decision: UiToolReviewDecision,
    pub label: String,
    pub description: String,
}

/// A finite UI selection that must be mapped to a matching journal request.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum UiToolReviewDecision {
    ApproveOnce,
    Reject,
}

impl UiToolReviewDecision {
    /// An identity for pointer selection, separate from Scheme-owned labels.
    pub fn selection_id(self) -> &'static str {
        match self {
            Self::ApproveOnce => "tool-review/approve-once",
            Self::Reject => "tool-review/reject",
        }
    }
}

impl UiCommand {
    pub fn accepts_arguments(&self) -> bool {
        self.arguments != UiCommandArguments::None
    }
}

/// Scheme-declared argument grammar, checked before invoking a host capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiCommandArguments {
    None,
    Required,
    Optional,
}

/// The finite Rust-owned capability catalog. Names and aliases stay in Scheme.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiCommandCapability {
    Models,
    Model,
    Sessions,
    Session,
    Inspect,
    History,
    Workspace,
    ContextBudget,
    Runtime,
    OpenMenu,
    SelectProfile,
    SelectBuffer,
    EnterTransient,
    KillProcess,
    Envrc,
    ToggleThinking,
    Clear,
    Helm,
    /// A Scheme catalog entry awaiting publication admission.
    Deferred,
}
