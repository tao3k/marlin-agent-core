// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Host-neutral UI state and transition contract.

use gerbil_scheme::{NativeError, RootedCallError, RootedSchemeObject};

// Three core regions, sixteen contributed regions, and the status region.
pub(crate) const MAX_REGIONS: usize = 20;
pub(crate) const MAX_HOST_STATUS: usize = 8;
// Five visible slots, three transport observations and eight host contributions.
pub(crate) const MAX_STATUS: usize = 16;
pub(crate) const MAX_EXTENSIONS: usize = 16;
pub(crate) const MAX_CONTENT_ITEMS: usize = 16;
pub(crate) const MAX_METHOD_BUNDLES: usize = 16;
pub(crate) const MAX_COMMANDS: usize = 32;
pub(crate) const MAX_TEXT_CHARS: usize = 256;
pub(crate) const MAX_ID_BYTES: usize = 1024;

// Keep protocol admission and renderer capacity tied to the selected shape.
const _: () = assert!(MAX_STATUS == 8 + MAX_HOST_STATUS);
const _: () = assert!(MAX_REGIONS == 3 + MAX_EXTENSIONS + 1);

/// A UI component's domain identity, not a Scheme root or object address.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UiComponentId(String);

/// A displayable command name; this value does not authorize a handler.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UiCommandName(String);

impl UiCommandName {
    /// Check a partial command without scanning past the admitted name limit.
    pub fn is_valid_prefix(value: &str) -> bool {
        value.len() <= 32
            && value.starts_with('/')
            && value.as_bytes()[1..]
                .iter()
                .all(|byte| byte.is_ascii_lowercase() || *byte == b'-')
    }

    pub fn new(value: &str) -> Result<Self, UiProjectionError> {
        if value.len() < 2 || !Self::is_valid_prefix(value) {
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
    pub accepts_arguments: bool,
}

impl UiComponentId {
    /// Construct a bounded, nonempty domain identity.
    pub fn new(value: &str) -> Result<Self, UiProjectionError> {
        if value.is_empty() || value.len() > MAX_ID_BYTES {
            return Err(UiProjectionError::Invalid("invalid identity length"));
        }
        Ok(Self(value.to_owned()))
    }
    /// The identity's spelling. Identity admission rejects duplicate spellings.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One declarative workbench region, without authority to execute effects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiRegion {
    Transcript,
    Composer,
    Commands(Vec<UiCommand>),
    Status(Vec<UiStatusItem>),
    Extension {
        id: UiComponentId,
        label: String,
        focusable: bool,
        content: Vec<UiContent>,
    },
}

/// Bounded, display-only content from a Scheme extension component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiContent {
    Text(String),
    Field { label: String, value: String },
}

/// One ordered Modeline contribution and its active tone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiStatusItem {
    pub id: UiComponentId,
    pub label: String,
    pub compact_label: String,
    pub tone: UiStatusTone,
    pub active_tone: UiStatusTone,
}

impl UiStatusItem {
    /// A host fact has no authority to define modeline presentation.
    pub fn host(id: UiComponentId, label: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            id,
            compact_label: label.clone(),
            label,
            tone: UiStatusTone::Muted,
            active_tone: UiStatusTone::Muted,
        }
    }
}

/// Semantic presentation tone declared by the selected Scheme module.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiStatusTone {
    Accent,
    Info,
    Success,
    Warning,
    Muted,
}

/// Bounded, Scheme-selected geometry policy. Terminal clipping stays native.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiLayoutPolicy {
    pub min_transcript_rows: u16,
    pub max_chatbox_rows: u16,
    pub max_command_rows: u16,
    pub extension_column_min: u16,
    pub extension_panel_columns: u16,
}

impl Default for UiLayoutPolicy {
    fn default() -> Self {
        Self {
            min_transcript_rows: 2,
            max_chatbox_rows: 10,
            max_command_rows: 4,
            extension_column_min: 100,
            extension_panel_columns: 36,
        }
    }
}

/// An RGB color selected by Scheme and admitted at the native boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiRgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl UiRgb {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

/// Shared presentation palette for the native terminal adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiTheme {
    pub canvas: UiRgb,
    pub ink: UiRgb,
    pub bar: UiRgb,
    pub accent: UiRgb,
    pub info: UiRgb,
    pub success: UiRgb,
    pub warning: UiRgb,
    pub muted: UiRgb,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            canvas: UiRgb::new(40, 44, 52),
            ink: UiRgb::new(187, 194, 207),
            bar: UiRgb::new(28, 31, 36),
            accent: UiRgb::new(198, 120, 221),
            info: UiRgb::new(81, 175, 239),
            success: UiRgb::new(152, 190, 101),
            warning: UiRgb::new(236, 190, 123),
            muted: UiRgb::new(115, 121, 126),
        }
    }
}

/// Host display facts and already-rooted Scheme extension contributions.
/// The selected publication and extension provenance remain the VM/module
/// owner's responsibility.
pub struct UiOpenFacts<'a, 'runtime> {
    pub model_label: &'a str,
    pub session_label: &'a str,
    pub status: &'a [UiStatusItem],
    pub extensions: &'a [RootedSchemeObject<'runtime>],
    pub method_bundles: &'a [RootedSchemeObject<'runtime>],
}

/// An owned candidate. Publication and effect admission remain host duties.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiWorkbench {
    pub model_label: String,
    pub session_label: String,
    pub regions: Vec<UiRegion>,
    pub layout: UiLayoutPolicy,
    pub theme: UiTheme,
    pub focus: UiComponentId,
}

impl UiWorkbench {
    pub fn commands(&self) -> &[UiCommand] {
        self.regions
            .iter()
            .find_map(|region| match region {
                UiRegion::Commands(commands) => Some(commands.as_slice()),
                _ => None,
            })
            .unwrap_or(&[])
    }

    /// Whether the selected region is an extension that accepts focus.
    pub fn focused_extension(&self) -> bool {
        self.regions.iter().any(|region| {
            matches!(region, UiRegion::Extension { id, focusable: true, .. } if id == &self.focus)
        })
    }

    /// Admit a Scheme successor before any host applies its proposed effect.
    /// Renderer-specific command handlers and geometry remain the host's gate.
    pub fn admit_successor(
        &self,
        next: &Self,
        event: UiEvent,
        intent: &UiIntent,
    ) -> Result<(), UiProjectionError> {
        let supported = matches!(
            (event, intent),
            (
                UiEvent::NextFocus | UiEvent::PreviousFocus | UiEvent::Dismiss,
                UiIntent::Focus(_)
            ) | (UiEvent::ScrollUp, UiIntent::Scroll(UiScrollDirection::Up))
                | (
                    UiEvent::ScrollDown,
                    UiIntent::Scroll(UiScrollDirection::Down)
                )
        ) || (event == UiEvent::Submit
            && match intent {
                UiIntent::SubmitDraft => self.focus.as_str() == "composer",
                UiIntent::Focus(_) => self.focused_extension(),
                UiIntent::Scroll(_) => false,
            });
        if !supported {
            return Err(UiProjectionError::Invalid(
                "UI intent does not match the event",
            ));
        }
        if self.model_label != next.model_label
            || self.session_label != next.session_label
            || self.regions != next.regions
            || self.layout != next.layout
            || self.theme != next.theme
        {
            return Err(UiProjectionError::Invalid(
                "UI event changed the admitted workbench",
            ));
        }
        let expected = match intent {
            UiIntent::Focus(id) => id,
            UiIntent::SubmitDraft | UiIntent::Scroll(_) => &self.focus,
        };
        if &next.focus != expected {
            return Err(UiProjectionError::Invalid(
                "UI intent and successor focus disagree",
            ));
        }
        Ok(())
    }
}

/// Input events accepted by the Scheme UI dispatcher.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiEvent {
    NextFocus,
    PreviousFocus,
    Submit,
    Dismiss,
    ScrollUp,
    ScrollDown,
}

/// Direction for moving through a visible collection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiBrowseDirection {
    Next,
    Previous,
}

/// A presentation proposal; only the runtime can authorize its effects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiIntent {
    SubmitDraft,
    Focus(UiComponentId),
    Scroll(UiScrollDirection),
}

/// Direction for scrolling a UI region.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiScrollDirection {
    Up,
    Down,
}

/// A rejected or failed native Scheme projection.
#[derive(Debug)]
pub enum UiProjectionError {
    Native(NativeError),
    Call(RootedCallError),
    Invalid(&'static str),
}
impl std::fmt::Display for UiProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native(error) => write!(formatter, "{error}"),
            Self::Call(error) => write!(formatter, "{error}"),
            Self::Invalid(reason) => write!(formatter, "invalid UI projection: {reason}"),
        }
    }
}
impl std::error::Error for UiProjectionError {}
impl From<RootedCallError> for UiProjectionError {
    fn from(error: RootedCallError) -> Self {
        Self::Call(error)
    }
}
impl From<NativeError> for UiProjectionError {
    fn from(error: NativeError) -> Self {
        Self::Native(error)
    }
}
