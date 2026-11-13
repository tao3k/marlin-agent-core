// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Host-neutral UI state and transition contract.

use gerbil_scheme::{NativeError, RootedCallError, RootedSchemeObject};
use marlin_agent_protocol::HumanDecision;

// Three core regions, one built-in buffer, sixteen contributions, and status.
pub(crate) const MAX_REGIONS: usize = 21;
pub(crate) const MAX_HOST_STATUS: usize = 8;
// Five visible slots, ten runtime/transport observations and eight host contributions.
pub(crate) const MODELINE_SLOTS: usize = 15;
pub(crate) const MAX_STATUS: usize = 23;
pub(crate) const MAX_EXTENSIONS: usize = 16;
pub(crate) const MAX_CONTENT_ITEMS: usize = 16;
pub(crate) const MAX_METHOD_BUNDLES: usize = 16;
pub(crate) const MAX_COMMANDS: usize = 256;
pub(crate) const MAX_TEXT_CHARS: usize = 256;
pub(crate) const MAX_ID_BYTES: usize = 1024;

// Keep protocol admission and renderer capacity tied to the selected shape.
const _: () = assert!(MAX_STATUS == MODELINE_SLOTS + MAX_HOST_STATUS);
const _: () = assert!(MAX_REGIONS == 3 + 1 + MAX_EXTENSIONS + 1);

/// A UI component's domain identity, not a Scheme root or object address.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UiComponentId(String);

/// A displayable command name; this value does not authorize a handler.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UiCommandName(String);

/// Stable Scheme buffer identity. Content remains native and is never copied
/// through the UI session merely to switch a window.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UiBufferId(String);

impl UiBufferId {
    pub fn new(value: &str) -> Result<Self, UiProjectionError> {
        if !UiCommandName::is_valid_prefix(value) || value.is_empty() {
            return Err(UiProjectionError::Invalid("invalid UI buffer identity"));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One window binds a buffer; the buffer's contents stay with their owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiWindow {
    pub id: UiComponentId,
    pub buffer: UiBufferId,
}

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
    LineStart,
    LineEnd,
}

/// One Scheme-declared shortcut and the admitted host command it invokes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiKeyBinding {
    pub sequence: Vec<UiKeyToken>,
    pub command: UiKeyCommand,
}

/// Scheme-owned frame topology projected into a bounded native snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiFrame {
    pub id: UiComponentId,
    pub windows: Vec<UiWindow>,
    pub selected_window: UiComponentId,
    pub minibuffer: UiComponentId,
    pub buffers: Vec<UiBufferId>,
}

impl UiFrame {
    pub fn selected_buffer(&self) -> Option<&UiBufferId> {
        self.windows
            .iter()
            .find(|window| window.id == self.selected_window)
            .map(|window| &window.buffer)
    }
}

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
    ToggleThinking,
    Clear,
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
    /// Static text or a dynamic Modeline template with one `{value}` slot.
    pub label: String,
    /// Compact presentation with the same slot rule for dynamic items.
    pub compact_label: String,
    pub tone: UiStatusTone,
    pub active_tone: UiStatusTone,
    /// Minimum terminal width at which a dynamic contribution is shown.
    pub min_width: u16,
    /// Use the compact label below this terminal width.
    pub compact_below: u16,
    /// Larger values retain optional contributions first when space is scarce.
    pub keep_priority: u8,
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
            min_width: 0,
            compact_below: 0,
            keep_priority: 0,
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

/// One Scheme-declared frame mapped to Ratatui constraints by the host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiFrameSpec {
    pub min_transcript_rows: u16,
    /// Zero means measure the expandable editor from its content.
    pub fixed_editor_rows: u16,
    pub footer_rows: u16,
    pub extensions: bool,
    pub bordered: bool,
    pub prompt: String,
}

/// Bounded, Scheme-selected geometry policy. Terminal clipping stays native.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiLayoutPolicy {
    pub message_frame: UiFrameSpec,
    pub command_frame: UiFrameSpec,
    pub max_chatbox_rows: u16,
    pub max_command_rows: u16,
    pub transcript_policy: UiTranscriptPolicy,
    /// Ordered frame regions, authored by the selected Scheme layout.
    pub frame_tree: crate::UiElement,
}

/// Scheme-selected compact conversation presentation; raw messages remain in history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiTranscriptPolicy {
    pub default_collapsed: bool,
    pub user_prefix: String,
    pub assistant_role: String,
    pub thinking_collapsed_role: String,
    pub thinking_expanded_role: String,
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
    pub frame: UiFrame,
    pub keymap: Vec<UiKeyBinding>,
    pub transients: Vec<UiTransient>,
}

impl UiWorkbench {
    pub fn key_command(&self, sequence: &[&str]) -> Option<UiKeyCommand> {
        self.keymap
            .iter()
            .find(|binding| {
                binding.sequence.len() == sequence.len()
                    && binding
                        .sequence
                        .iter()
                        .zip(sequence)
                        .all(|(left, right)| left.as_str() == *right)
            })
            .map(|binding| binding.command.clone())
    }

    pub fn key_prefix(&self, sequence: &[&str]) -> bool {
        self.keymap.iter().any(|binding| {
            binding.sequence.len() > sequence.len()
                && binding
                    .sequence
                    .iter()
                    .zip(sequence)
                    .all(|(left, right)| left.as_str() == *right)
        })
    }

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
        let profile_change = matches!(&event, UiEvent::SelectProfile(_));
        let buffer_change = matches!(&event, UiEvent::SelectBuffer(_));
        let supported = matches!(
            (&event, intent),
            (
                UiEvent::NextFocus | UiEvent::PreviousFocus | UiEvent::Dismiss,
                UiIntent::Focus(_)
            ) | (UiEvent::ScrollUp, UiIntent::Scroll(UiScrollDirection::Up))
                | (
                    UiEvent::ScrollDown,
                    UiIntent::Scroll(UiScrollDirection::Down)
                )
        ) || (profile_change
            && matches!(intent, UiIntent::Focus(id) if id == &self.focus))
            || (buffer_change
                && matches!((&event, intent), (UiEvent::SelectBuffer(requested), UiIntent::SelectBuffer(id))
                    if requested == id && next.frame.selected_buffer() == Some(id)))
            || (event == UiEvent::Submit
                && match intent {
                    UiIntent::SubmitDraft => self.focus.as_str() == "composer",
                    UiIntent::Focus(_) => self.focused_extension(),
                    UiIntent::Scroll(_) | UiIntent::SelectBuffer(_) => false,
                });
        if !supported {
            return Err(UiProjectionError::Invalid(
                "UI intent does not match the event",
            ));
        }
        if self.model_label != next.model_label
            || self.session_label != next.session_label
            || if profile_change {
                self.layout.message_frame != next.layout.message_frame
                    || self.layout.command_frame != next.layout.command_frame
                    || self.layout.max_chatbox_rows != next.layout.max_chatbox_rows
                    || self.layout.max_command_rows != next.layout.max_command_rows
                    || self.layout.frame_tree != next.layout.frame_tree
            } else {
                self.layout != next.layout
            }
            || self.keymap != next.keymap
            || self.transients != next.transients
            || if buffer_change {
                self.frame.id != next.frame.id
                    || self.frame.selected_window != next.frame.selected_window
                    || self.frame.minibuffer != next.frame.minibuffer
                    || self.frame.buffers != next.frame.buffers
                    || self.frame.windows.len() != next.frame.windows.len()
                    || self
                        .frame
                        .windows
                        .iter()
                        .zip(&next.frame.windows)
                        .any(|(old, new)| {
                            old.id != new.id
                                || (old.id != self.frame.selected_window
                                    && old.buffer != new.buffer)
                        })
            } else {
                self.frame != next.frame
            }
            || if profile_change {
                !profile_regions_match(&self.regions, &next.regions)
            } else {
                self.regions != next.regions || self.theme != next.theme
            }
        {
            return Err(UiProjectionError::Invalid(
                "UI event changed the admitted workbench",
            ));
        }
        let expected = match intent {
            UiIntent::Focus(id) => id,
            UiIntent::SubmitDraft | UiIntent::Scroll(_) | UiIntent::SelectBuffer(_) => &self.focus,
        };
        if &next.focus != expected {
            return Err(UiProjectionError::Invalid(
                "UI intent and successor focus disagree",
            ));
        }
        Ok(())
    }
}

fn profile_regions_match(previous: &[UiRegion], next: &[UiRegion]) -> bool {
    previous.len() == next.len()
        && previous
            .iter()
            .zip(next)
            .all(|(left, right)| match (left, right) {
                (UiRegion::Status(a), UiRegion::Status(b)) => profile_status_match(a, b),
                _ => left == right,
            })
}

fn profile_status_match(previous: &[UiStatusItem], next: &[UiStatusItem]) -> bool {
    fn unique_modeline_ids(items: &[UiStatusItem]) -> Option<([&str; MODELINE_SLOTS], usize)> {
        let mut ids = [""; MODELINE_SLOTS];
        let mut count = 0;
        for item in items {
            if item.id.as_str().starts_with("modeline/") {
                if count == MODELINE_SLOTS {
                    return None;
                }
                ids[count] = item.id.as_str();
                count += 1;
            }
        }
        ids[..count].sort_unstable();
        if ids[..count].windows(2).any(|pair| pair[0] == pair[1]) {
            return None;
        }
        Some((ids, count))
    }

    if previous.len() != next.len() {
        return false;
    }
    let (Some((old_ids, old_count)), Some((new_ids, new_count))) =
        (unique_modeline_ids(previous), unique_modeline_ids(next))
    else {
        return false;
    };
    old_count == new_count
        && old_ids[..old_count] == new_ids[..new_count]
        && previous
            .iter()
            .filter(|item| !item.id.as_str().starts_with("modeline/"))
            .eq(next
                .iter()
                .filter(|item| !item.id.as_str().starts_with("modeline/")))
}

/// Input events accepted by the Scheme UI dispatcher.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiEvent {
    NextFocus,
    PreviousFocus,
    Submit,
    Dismiss,
    ScrollUp,
    ScrollDown,
    SelectProfile(UiProfileName),
    SelectBuffer(UiBufferId),
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
    pub steps: u8,
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
