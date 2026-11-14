// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Scheme-owned key sequences and transient input mapped to typed host actions.

use super::{UiCommandName, UiProjectionError};

/// A terminal-independent key sequence token admitted from Scheme.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UiKeyToken(String);

impl UiKeyToken {
    pub fn new(value: &str) -> Result<Self, UiProjectionError> {
        if value.is_empty()
            || value.len() > 16
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(UiProjectionError::Invalid("invalid UI key token"));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Host capabilities are fixed; Scheme owns their bindings and sequences.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiKeyCommand {
    OpenCommands,
    Cancel,
    Edit(UiEditorAction),
    Invoke(UiCommandName),
    /// Switch an editor mode; `cancel` preserves the active Escape action.
    Mode {
        mode: UiKeyMode,
        cancel: bool,
    },
}

/// The native editor's input interpretation mode.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UiKeyMode {
    Insert,
    Normal,
}

/// Scheme selects whether a binding is global or applies in one editor mode.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UiKeyScope {
    Global,
    Mode(UiKeyMode),
}

/// Identity of a temporary Scheme-declared keymap.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UiTransientId(String);

impl UiTransientId {
    pub fn new(value: &str) -> Result<Self, UiProjectionError> {
        if value.is_empty()
            || value.len() > 20
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(UiProjectionError::Invalid("invalid transient identity"));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A bounded transient keymap projected from the selected Scheme module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiTransient {
    pub id: UiTransientId,
    pub hint: crate::UiElement,
    pub warning_hint: crate::UiElement,
    pub foreign: UiTransientForeign,
    pub heads: Vec<UiTransientHead>,
}

/// Action for a key outside the active transient keymap.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiTransientForeign {
    Exit,
    Warn,
    Run,
}

/// One key and command exposed by a temporary keymap.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiTransientHead {
    pub key: UiKeyToken,
    pub command: Option<UiCommandName>,
    pub label: String,
    pub exit: bool,
}

/// Native editor actions bound by the selected Scheme keymap at UI open.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiEditorAction {
    HistorySearch,
    PreviousPrompt,
    NextPrompt,
    InsertNewline,
    BackwardChar,
    ForwardChar,
    DeleteChar,
    PreviousLine,
    NextLine,
    AppendInsert,
    NextWord,
    PreviousWord,
    DeleteLine,
    Undo,
    LineStart,
    LineEnd,
}

/// One Scheme-declared shortcut and the admitted host command it invokes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiKeyBinding {
    pub scope: UiKeyScope,
    pub sequence: Vec<UiKeyToken>,
    pub command: UiKeyCommand,
}

