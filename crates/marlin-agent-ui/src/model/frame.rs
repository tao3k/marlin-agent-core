// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Owned UI frame topology, geometry and presentation values.

use std::sync::Arc;

use crate::UiElement;

use super::{MAX_ID_BYTES, UiCommand, UiCommandName, UiProjectionError};

/// A UI component's domain identity, shared across projected hit regions.
/// It is not a Scheme root or object address.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UiComponentId(Arc<str>);

/// A Scheme-selected host capability for scrolling a region.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UiScrollTarget {
    id: UiComponentId,
    step: u16,
}

impl UiScrollTarget {
    pub fn new(value: &str, step: u16) -> Result<Self, UiProjectionError> {
        Self::from_id(UiComponentId::new(value)?, step)
    }

    pub fn from_id(id: UiComponentId, step: u16) -> Result<Self, UiProjectionError> {
        if !(1..=32).contains(&step) {
            return Err(UiProjectionError::Invalid("invalid UI scroll step"));
        }
        Ok(Self { id, step })
    }

    pub fn as_str(&self) -> &str {
        self.id.as_str()
    }

    pub fn step(&self) -> u16 {
        self.step
    }
}

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

impl UiComponentId {
    /// Construct a bounded, nonempty domain identity.
    pub fn new(value: &str) -> Result<Self, UiProjectionError> {
        if value.is_empty() || value.len() > MAX_ID_BYTES {
            return Err(UiProjectionError::Invalid("invalid identity length"));
        }
        Ok(Self(value.into()))
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
        content: UiElement,
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
    pub echo: UiEchoPolicy,
    /// Ordered frame regions, authored by the selected Scheme layout.
    pub frame_tree: crate::UiElement,
    pub regions: UiFrameRegions,
}

/// Scheme-selected transient feedback for the shared minibuffer surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiEchoPolicy {
    pub view: crate::UiElement,
    pub timeout: std::time::Duration,
    pub clear_on_input: bool,
}

impl Default for UiEchoPolicy {
    fn default() -> Self {
        Self {
            view: crate::UiElement::Template {
                background: crate::UiBackground::None,
                spans: Vec::new(),
            },
            timeout: std::time::Duration::ZERO,
            clear_on_input: false,
        }
    }
}

/// A terminal mechanism, independent of the Scheme region that provides it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiFrameRole {
    Transcript,
    Extensions,
    Modeline,
    Editor,
    Candidates,
    Minibuffer,
    MinibufferHover,
}

impl UiFrameRole {
    pub const ALL: [Self; 7] = [
        Self::Transcript,
        Self::Extensions,
        Self::Modeline,
        Self::Editor,
        Self::Candidates,
        Self::Minibuffer,
        Self::MinibufferHover,
    ];

    /// Match against the seven names in the terminal role contract.
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|role| role.as_str() == name)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transcript => "transcript",
            Self::Extensions => "extensions",
            Self::Modeline => "modeline",
            Self::Editor => "editor",
            Self::Candidates => "candidates",
            Self::Minibuffer => "minibuffer",
            Self::MinibufferHover => "minibuffer-hover",
        }
    }
}

/// Admitted role bindings in `UiFrameRole::ALL` order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiFrameRegions([UiComponentId; 7]);

impl UiFrameRegions {
    pub fn new(ids: [UiComponentId; 7]) -> Result<Self, UiProjectionError> {
        for (index, id) in ids.iter().enumerate() {
            if ids[..index].contains(id) {
                return Err(UiProjectionError::Invalid("duplicate frame role binding"));
            }
        }
        Ok(Self(ids))
    }

    pub fn id(&self, role: UiFrameRole) -> &UiComponentId {
        &self.0[role as usize]
    }
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
