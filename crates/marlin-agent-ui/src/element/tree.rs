// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Owned UI elements and their terminal-independent layout facts.

use crate::{UiComponentId, UiScrollTarget};

/// A reusable declarative tree. Product modules compose these primitives.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiElement {
    Text(Vec<UiSpan>),
    /// An opaque host-provided visual source selected by a Scheme component.
    Slot(UiSlotSource),
    /// A titled presentation around a host-backed visual source.
    Panel {
        border_tone: Option<UiTone>,
        field_template: Box<UiElement>,
        title: Option<Box<UiElement>>,
        child: Box<UiElement>,
    },
    /// A styled line whose named fields are bound by the host.
    Template {
        background: UiBackground,
        spans: Vec<UiTemplateSpan>,
    },
    /// A one-line template with an editable trailing host-bound field.
    Prompt {
        field: UiComponentId,
        template: Box<UiElement>,
    },
    /// A single-row flow of Scheme-declared, priority-aware segments.
    Bar {
        background: UiBackground,
        right_divisor: u16,
        segments: Vec<UiBarSegment>,
    },
    /// A repeated row and its Scheme-declared window offset.
    List {
        id: UiComponentId,
        source: UiListSource,
        offset: UiSize,
        child: Box<UiElement>,
    },
    /// A host-supplied editable value with Scheme-selected presentation.
    Input {
        bordered: bool,
        background: UiBackground,
        prefix: Option<Box<UiElement>>,
        title: Option<UiComponentId>,
        active: UiComponentId,
        trailing: Option<UiInputAdornment>,
    },
    Rows(Vec<UiElement>),
    Columns(Vec<UiElement>),
    /// The first true named fact selects its view; otherwise use the fallback.
    Choice {
        cases: Vec<(UiComponentId, UiElement)>,
        fallback: Box<UiElement>,
    },
    /// A layout constraint supplied by the module that composes the tree.
    Sized {
        size: UiSize,
        child: Box<UiElement>,
    },
    /// An interactive area whose identity is owned by the Scheme module.
    Region {
        id: UiComponentId,
        scroll_target: Option<UiScrollTarget>,
        child: Box<UiElement>,
    },
}

/// A one-line view sharing the editable value's cursor row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiInputAdornment {
    pub cells: u16,
    pub view: Box<UiElement>,
}

/// A list's data source is independent of its widget identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiListSource {
    Conversation,
    Messages,
    Workspace,
    Review,
    Candidates,
    Extensions,
    Rows(Vec<UiListValue>),
}

/// A visual capability whose placement is selected by the Scheme tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiSlotSource {
    SelectedBuffer,
    FocusedExtension,
    RuntimePolicy,
    TransientHint,
}

/// A Scheme-authored row for the same list mapper used by host capabilities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiListValue {
    pub id: Option<UiComponentId>,
    pub fields: Vec<(UiComponentId, String)>,
}

/// Placement of a Scheme-declared modeline segment within its terminal row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiBarSide {
    Left,
    Right,
}

/// One modeline segment with a priority and optional visibility condition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiBarSegment {
    pub side: UiBarSide,
    pub priority: u8,
    pub min_width: u16,
    pub condition: Option<UiComponentId>,
    pub child: Box<UiElement>,
    /// An existing command name activated by a click on the painted segment.
    pub command: Option<UiComponentId>,
}

/// A fixed, flexible, or host-bound extent on the containing layout axis.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiSize {
    Cells(u16),
    Flex(u16),
    Binding(UiComponentId),
    /// A cell measurement capped by another host-supplied measurement.
    Bounded {
        value: UiComponentId,
        limit: UiComponentId,
    },
    /// Saturating cell arithmetic, composed by the Scheme layout.
    Min(Box<UiSize>, Box<UiSize>),
    Max(Box<UiSize>, Box<UiSize>),
    Add(Box<UiSize>, Box<UiSize>),
    Subtract(Box<UiSize>, Box<UiSize>),
    /// The first true Scheme-selected fact chooses the size.
    Choice {
        cases: Vec<(UiComponentId, UiSize)>,
        fallback: Box<UiSize>,
    },
}

/// One literal or dynamically bound span in a declarative line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiTemplateSpan {
    pub value: UiTemplateValue,
    pub tone: UiTone,
    pub bold: bool,
}

/// Text data in a declarative line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiTemplateValue {
    Literal(String),
    Binding(UiComponentId),
    /// A bound value padded to the Scheme-declared minimum cell width.
    Column {
        id: UiComponentId,
        width: UiSize,
    },
    Tail {
        id: UiComponentId,
        width: UiSize,
    },
}

/// One styled text run in a host-neutral UI element.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiSpan {
    pub text: String,
    pub tone: UiTone,
}

/// Semantic color role resolved by the selected UI theme.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiTone {
    Normal,
    Canvas,
    Accent,
    Info,
    Success,
    Warning,
    Muted,
}

/// Background selected from the active theme for a template line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiBackground {
    None,
    Canvas,
    Bar,
    Tone(UiTone),
}

impl UiElement {
    /// Profile switches may restyle presentation without changing its data bindings.
    pub(crate) fn same_profile_structure(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bar { .. }, Self::Bar { .. }) => true,
            (Self::Template { spans: a, .. }, Self::Template { spans: b, .. }) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.value == y.value)
            }
            (
                Self::Region {
                    id: a,
                    scroll_target: x,
                    child: c,
                },
                Self::Region {
                    id: b,
                    scroll_target: y,
                    child: d,
                },
            ) => a == b && x == y && c.same_profile_structure(d),
            (Self::Sized { size: a, child: c }, Self::Sized { size: b, child: d }) => {
                a == b && c.same_profile_structure(d)
            }
            (Self::Rows(a), Self::Rows(b)) | (Self::Columns(a), Self::Columns(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.same_profile_structure(y))
            }
            (
                Self::Choice {
                    cases: a,
                    fallback: x,
                },
                Self::Choice {
                    cases: b,
                    fallback: y,
                },
            ) => {
                a.len() == b.len()
                    && a.iter()
                        .zip(b)
                        .all(|((ka, va), (kb, vb))| ka == kb && va.same_profile_structure(vb))
                    && x.same_profile_structure(y)
            }
            (
                Self::List {
                    id: a,
                    source: sa,
                    offset: x,
                    child: c,
                },
                Self::List {
                    id: b,
                    source: sb,
                    offset: y,
                    child: d,
                },
            ) => a == b && sa == sb && x == y && c.same_profile_structure(d),
            (
                Self::Panel {
                    border_tone: a,
                    field_template: f,
                    title: t,
                    child: c,
                },
                Self::Panel {
                    border_tone: b,
                    field_template: g,
                    title: u,
                    child: d,
                },
            ) => a == b && f == g && t == u && c.same_profile_structure(d),
            _ => self == other,
        }
    }
    /// Find a named region's contents in a declarative tree.
    pub fn region_child(&self, name: &str) -> Option<&UiElement> {
        match self {
            Self::Region { id, child, .. } if id.as_str() == name => Some(child),
            Self::Region { child, .. } => child.region_child(name),
            Self::Sized { child, .. } => child.region_child(name),
            Self::Panel { child, .. } => child.region_child(name),
            Self::List { child, .. } => child.region_child(name),
            Self::Rows(children) | Self::Columns(children) => {
                children.iter().find_map(|child| child.region_child(name))
            }
            Self::Choice { cases, fallback } => cases
                .iter()
                .find_map(|(_, child)| child.region_child(name))
                .or_else(|| fallback.region_child(name)),
            Self::Text(_)
            | Self::Slot(_)
            | Self::Template { .. }
            | Self::Prompt { .. }
            | Self::Input { .. }
            | Self::Bar { .. } => None,
        }
    }
    /// Natural line count before wrapping or clipping to the terminal.
    pub fn intrinsic_rows(&self) -> usize {
        match self {
            Self::Text(spans) => 1usize.saturating_add(
                spans
                    .iter()
                    .map(|span| span.text.bytes().filter(|byte| *byte == b'\n').count())
                    .sum::<usize>(),
            ),
            Self::Template { .. }
            | Self::Prompt { .. }
            | Self::Input { .. }
            | Self::Slot(_)
            | Self::Bar { .. } => 1,
            Self::Rows(children) => children.iter().fold(0usize, |rows, child| {
                rows.saturating_add(child.intrinsic_rows())
            }),
            Self::Columns(children) => children.iter().map(Self::intrinsic_rows).max().unwrap_or(0),
            Self::Choice { cases, fallback } => cases
                .iter()
                .map(|(_, child)| child.intrinsic_rows())
                .chain(std::iter::once(fallback.intrinsic_rows()))
                .max()
                .unwrap_or(0),
            Self::Region { child, .. } => child.intrinsic_rows(),
            Self::Sized { child, .. } => child.intrinsic_rows(),
            Self::Panel { child, .. } => child.intrinsic_rows(),
            Self::List { child, .. } => child.intrinsic_rows(),
        }
    }

    /// Natural height of the branch selected by the supplied facts.
    pub fn intrinsic_rows_with(&self, facts: &dyn Fn(&UiComponentId) -> bool) -> usize {
        match self {
            Self::Choice { cases, fallback } => cases
                .iter()
                .find_map(|(fact, child)| facts(fact).then_some(child))
                .unwrap_or(fallback)
                .intrinsic_rows_with(facts),
            Self::Rows(children) => children.iter().fold(0usize, |rows, child| {
                rows.saturating_add(child.intrinsic_rows_with(facts))
            }),
            Self::Columns(children) => children
                .iter()
                .map(|child| child.intrinsic_rows_with(facts))
                .max()
                .unwrap_or(0),
            Self::Region { child, .. } => child.intrinsic_rows_with(facts),
            Self::Sized { child, .. } => child.intrinsic_rows_with(facts),
            Self::Panel { child, .. } => child.intrinsic_rows_with(facts),
            Self::List { child, .. } => child.intrinsic_rows_with(facts),
            Self::Text(_)
            | Self::Slot(_)
            | Self::Template { .. }
            | Self::Prompt { .. }
            | Self::Input { .. }
            | Self::Bar { .. } => self.intrinsic_rows(),
        }
    }
}
