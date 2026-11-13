// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Host-neutral UI primitives admitted from rooted Scheme values.

use gerbil_scheme::{IdentifierBuffer, RootedSchemeObject, SchemeScalar};
use std::collections::HashSet;

use crate::{
    UiComponentId, UiProjectionError, UiScrollTarget,
    projection::{identifier, vector},
};

/// A reusable declarative tree. Product modules compose these primitives.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiElement {
    Text(Vec<UiSpan>),
    /// An opaque host-provided visual source selected by a Scheme component.
    Slot(UiComponentId),
    /// A bordered presentation around a host-backed visual source.
    Panel {
        border_tone: UiTone,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiBarSide {
    Left,
    Right,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiBarSegment {
    pub side: UiBarSide,
    pub priority: u8,
    pub min_width: u16,
    pub condition: Option<UiComponentId>,
    pub child: Box<UiElement>,
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
    /// Profile switches may replace presentation-only bar segments, not geometry or input.
    pub(crate) fn same_profile_structure(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bar { .. }, Self::Bar { .. }) => true,
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
                    offset: x,
                    child: c,
                },
                Self::List {
                    id: b,
                    offset: y,
                    child: d,
                },
            ) => a == b && x == y && c.same_profile_structure(d),
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

struct Budget {
    nodes: usize,
    spans: usize,
    bytes: usize,
    regions: HashSet<UiComponentId>,
}

fn decode_size(
    kind: RootedSchemeObject<'_>,
    value: RootedSchemeObject<'_>,
    depth: usize,
    budget: &mut Budget,
    names: &mut IdentifierBuffer,
) -> Result<UiSize, UiProjectionError> {
    if depth >= 12 || budget.nodes == 0 {
        return Err(UiProjectionError::Invalid("UI size tree exceeds limit"));
    }
    budget.nodes -= 1;
    let kind = identifier(kind, names)?;
    match kind.as_str() {
        "cells" | "flex" => {
            let count = match value.as_ref().scalar()? {
                SchemeScalar::Fixnum(value) => u16::try_from(value)
                    .map_err(|_| UiProjectionError::Invalid("invalid UI size count"))?,
                _ => return Err(UiProjectionError::Invalid("invalid UI size count")),
            };
            Ok(if kind.as_str() == "cells" {
                UiSize::Cells(count)
            } else {
                UiSize::Flex(count)
            })
        }
        "binding" => Ok(UiSize::Binding(identifier(value, names)?)),
        "bounded" => {
            let pair = vector(value, 2, 2)?;
            Ok(UiSize::Bounded {
                value: identifier(pair.element(0)?, names)?,
                limit: identifier(pair.element(1)?, names)?,
            })
        }
        "min" | "max" | "add" | "subtract" => {
            let operands = vector(value, 2, 2)?;
            let mut operand = |index| {
                let size = vector(operands.element(index)?, 2, 2)?;
                decode_size(size.element(0)?, size.element(1)?, depth + 1, budget, names)
                    .map(Box::new)
            };
            let left = operand(0)?;
            let right = operand(1)?;
            Ok(match kind.as_str() {
                "min" => UiSize::Min(left, right),
                "max" => UiSize::Max(left, right),
                "add" => UiSize::Add(left, right),
                _ => UiSize::Subtract(left, right),
            })
        }
        "choice" => {
            let parts = vector(value, 2, 2)?;
            let entries = vector(parts.element(0)?, 0, 16)?;
            let mut seen = HashSet::new();
            let mut cases = Vec::with_capacity(entries.len());
            for index in 0..entries.len() {
                let entry = vector(entries.element(index)?, 2, 2)?;
                let fact = identifier(entry.element(0)?, names)?;
                if !seen.insert(fact.clone()) {
                    return Err(UiProjectionError::Invalid("duplicate UI size choice"));
                }
                let size = vector(entry.element(1)?, 2, 2)?;
                cases.push((
                    fact,
                    decode_size(size.element(0)?, size.element(1)?, depth + 1, budget, names)?,
                ));
            }
            let fallback = vector(parts.element(1)?, 2, 2)?;
            Ok(UiSize::Choice {
                cases,
                fallback: Box::new(decode_size(
                    fallback.element(0)?,
                    fallback.element(1)?,
                    depth + 1,
                    budget,
                    names,
                )?),
            })
        }
        _ => Err(UiProjectionError::Invalid("invalid UI size kind")),
    }
}

/// Project a bounded UI tree through gerbil-scheme's rooted object API.
pub fn decode_element(value: RootedSchemeObject<'_>) -> Result<UiElement, UiProjectionError> {
    let mut budget = Budget {
        // The budget includes size expressions as well as inline bar choices.
        // Keep the whole projection bounded while admitting a complete frame.
        nodes: 512,
        spans: 256,
        bytes: 32 * 1024,
        regions: HashSet::new(),
    };
    decode(value, 0, &mut budget, &mut IdentifierBuffer::default())
}

fn decode(
    value: RootedSchemeObject<'_>,
    depth: usize,
    budget: &mut Budget,
    names: &mut IdentifierBuffer,
) -> Result<UiElement, UiProjectionError> {
    if depth >= 12 || budget.nodes == 0 {
        return Err(UiProjectionError::Invalid("UI element tree exceeds limit"));
    }
    budget.nodes -= 1;
    let node = vector(value, 2, 2)?;
    let kind = identifier(node.element(0)?, names)?;
    let values = vector(node.element(1)?, 0, 64)?;
    match kind.as_str() {
        "bar" => {
            if values.len() != 3 {
                return Err(UiProjectionError::Invalid("invalid UI bar"));
            }
            let background = match identifier(values.element(0)?, names)?.as_str() {
                "none" => UiBackground::None,
                "canvas" => UiBackground::Canvas,
                "bar" => UiBackground::Bar,
                _ => UiBackground::Tone(decode_tone(values.element(0)?, names)?),
            };
            let right_divisor = match values.element(1)?.as_ref().scalar()? {
                SchemeScalar::Fixnum(value) if (1..=16).contains(&value) => value as u16,
                _ => return Err(UiProjectionError::Invalid("invalid UI bar divisor")),
            };
            let entries = vector(values.element(2)?, 0, 32)?;
            let mut segments = Vec::with_capacity(entries.len());
            for index in 0..entries.len() {
                let entry = vector(entries.element(index)?, 5, 5)?;
                let side = match identifier(entry.element(0)?, names)?.as_str() {
                    "left" => UiBarSide::Left,
                    "right" => UiBarSide::Right,
                    _ => return Err(UiProjectionError::Invalid("invalid UI bar side")),
                };
                let priority = match entry.element(1)?.as_ref().scalar()? {
                    SchemeScalar::Fixnum(value) if (0..=3).contains(&value) => value as u8,
                    _ => return Err(UiProjectionError::Invalid("invalid UI bar priority")),
                };
                let min_width = match entry.element(2)?.as_ref().scalar()? {
                    SchemeScalar::Fixnum(value) if (0..=256).contains(&value) => value as u16,
                    _ => return Err(UiProjectionError::Invalid("invalid UI bar width")),
                };
                let condition_value = entry.element(3)?;
                let condition = if matches!(
                    condition_value.as_ref().scalar(),
                    Ok(SchemeScalar::Boolean(false))
                ) {
                    None
                } else {
                    Some(identifier(condition_value, names)?)
                };
                let child = decode(entry.element(4)?, depth + 1, budget, names)?;
                if !inline_template(&child) {
                    return Err(UiProjectionError::Invalid("invalid UI bar segment"));
                }
                segments.push(UiBarSegment {
                    side,
                    priority,
                    min_width,
                    condition,
                    child: Box::new(child),
                });
            }
            Ok(UiElement::Bar {
                background,
                right_divisor,
                segments,
            })
        }
        "panel" => {
            if values.len() != 4 {
                return Err(UiProjectionError::Invalid("invalid UI panel"));
            }
            let border_tone = decode_tone(values.element(0)?, names)?;
            let field_template = decode(values.element(1)?, depth + 1, budget, names)?;
            if !inline_template(&field_template) {
                return Err(UiProjectionError::Invalid(
                    "invalid UI panel field template",
                ));
            }
            let title_value = values.element(2)?;
            let title = if matches!(
                title_value.as_ref().scalar(),
                Ok(SchemeScalar::Boolean(false))
            ) {
                None
            } else {
                let title = decode(title_value, depth + 1, budget, names)?;
                if !inline_template(&title) {
                    return Err(UiProjectionError::Invalid("invalid UI panel title"));
                }
                Some(Box::new(title))
            };
            let child = Box::new(decode(values.element(3)?, depth + 1, budget, names)?);
            Ok(UiElement::Panel {
                border_tone,
                field_template: Box::new(field_template),
                title,
                child,
            })
        }
        "slot" => {
            if values.len() != 1 {
                return Err(UiProjectionError::Invalid("invalid UI slot"));
            }
            Ok(UiElement::Slot(identifier(values.element(0)?, names)?))
        }
        "prompt" => {
            if values.len() != 2 {
                return Err(UiProjectionError::Invalid("invalid UI prompt"));
            }
            let field = identifier(values.element(0)?, names)?;
            let template = decode(values.element(1)?, depth + 1, budget, names)?;
            if !prompt_template(&template, &field) {
                return Err(UiProjectionError::Invalid("invalid UI prompt template"));
            }
            Ok(UiElement::Prompt {
                field,
                template: Box::new(template),
            })
        }
        "input" => {
            if values.len() != 5 {
                return Err(UiProjectionError::Invalid("invalid UI input"));
            }
            let bordered = match values.element(0)?.as_ref().scalar()? {
                SchemeScalar::Boolean(value) => value,
                _ => return Err(UiProjectionError::Invalid("invalid UI input border")),
            };
            let background = match identifier(values.element(1)?, names)?.as_str() {
                "none" => UiBackground::None,
                "canvas" => UiBackground::Canvas,
                "bar" => UiBackground::Bar,
                _ => return Err(UiProjectionError::Invalid("invalid UI input background")),
            };
            let optional = |index,
                            names: &mut IdentifierBuffer|
             -> Result<Option<UiComponentId>, UiProjectionError> {
                let value = values.element(index)?;
                if matches!(value.as_ref().scalar(), Ok(SchemeScalar::Boolean(false))) {
                    Ok(None)
                } else {
                    Ok(Some(identifier(value, names)?))
                }
            };
            let prefix_value = values.element(2)?;
            let prefix = if matches!(
                prefix_value.as_ref().scalar(),
                Ok(SchemeScalar::Boolean(false))
            ) {
                None
            } else {
                let prefix = decode(prefix_value, depth + 1, budget, names)?;
                if !inline_template(&prefix) {
                    return Err(UiProjectionError::Invalid("invalid UI input prefix"));
                }
                Some(Box::new(prefix))
            };
            let title = optional(3, names)?;
            let active = identifier(values.element(4)?, names)?;
            Ok(UiElement::Input {
                bordered,
                background,
                prefix,
                title,
                active,
            })
        }
        "sized" => {
            if values.len() != 3 {
                return Err(UiProjectionError::Invalid("invalid UI size"));
            }
            let size = decode_size(values.element(0)?, values.element(1)?, 0, budget, names)?;
            let child = Box::new(decode(values.element(2)?, depth + 1, budget, names)?);
            Ok(UiElement::Sized { size, child })
        }
        "list" => {
            if values.len() != 3 {
                return Err(UiProjectionError::Invalid("invalid UI list"));
            }
            let id = identifier(values.element(0)?, names)?;
            let size = vector(values.element(1)?, 2, 2)?;
            let offset = decode_size(size.element(0)?, size.element(1)?, 0, budget, names)?;
            let child = Box::new(decode(values.element(2)?, depth + 1, budget, names)?);
            Ok(UiElement::List { id, offset, child })
        }
        "template" => {
            if values.len() != 2 {
                return Err(UiProjectionError::Invalid("invalid UI template"));
            }
            let background_name = identifier(values.element(0)?, names)?;
            let background = match background_name.as_str() {
                "none" => UiBackground::None,
                "canvas" => UiBackground::Canvas,
                "bar" => UiBackground::Bar,
                _ => UiBackground::Tone(decode_tone(values.element(0)?, names)?),
            };
            let entries = vector(values.element(1)?, 0, 64)?;
            if entries.len() > budget.spans {
                return Err(UiProjectionError::Invalid("UI span count exceeds limit"));
            }
            budget.spans -= entries.len();
            let mut spans = Vec::with_capacity(entries.len());
            for index in 0..entries.len() {
                let entry = vector(entries.element(index)?, 4, 5)?;
                let source = identifier(entry.element(0)?, names)?;
                let tone = decode_tone(entry.element(1)?, names)?;
                let bold = match entry.element(2)?.as_ref().scalar()? {
                    SchemeScalar::Boolean(value) => value,
                    _ => return Err(UiProjectionError::Invalid("invalid UI template weight")),
                };
                let value = match source.as_str() {
                    "literal" => {
                        if entry.len() != 4 {
                            return Err(UiProjectionError::Invalid("invalid UI template span"));
                        }
                        let text = entry.element(3)?.into_string()?;
                        if text.len().into_result()? > budget.bytes {
                            return Err(UiProjectionError::Invalid("UI text exceeds limit"));
                        }
                        let text = text.to_string().into_result()?;
                        budget.bytes = budget
                            .bytes
                            .checked_sub(text.len())
                            .ok_or(UiProjectionError::Invalid("UI text exceeds limit"))?;
                        if text.chars().any(char::is_control) {
                            return Err(UiProjectionError::Invalid("invalid UI template text"));
                        }
                        UiTemplateValue::Literal(text)
                    }
                    "binding" => {
                        if entry.len() != 4 {
                            return Err(UiProjectionError::Invalid("invalid UI template span"));
                        }
                        UiTemplateValue::Binding(identifier(entry.element(3)?, names)?)
                    }
                    "column" => {
                        if entry.len() != 5 {
                            return Err(UiProjectionError::Invalid("invalid UI template column"));
                        }
                        let id = identifier(entry.element(3)?, names)?;
                        let size = vector(entry.element(4)?, 2, 2)?;
                        let width =
                            decode_size(size.element(0)?, size.element(1)?, 0, budget, names)?;
                        UiTemplateValue::Column { id, width }
                    }
                    "tail" => {
                        if entry.len() != 5 {
                            return Err(UiProjectionError::Invalid("invalid UI template tail"));
                        }
                        let id = identifier(entry.element(3)?, names)?;
                        let size = vector(entry.element(4)?, 2, 2)?;
                        let width =
                            decode_size(size.element(0)?, size.element(1)?, 0, budget, names)?;
                        UiTemplateValue::Tail { id, width }
                    }
                    _ => return Err(UiProjectionError::Invalid("invalid UI template source")),
                };
                spans.push(UiTemplateSpan { value, tone, bold });
            }
            Ok(UiElement::Template { background, spans })
        }
        "region" => {
            if values.len() != 3 {
                return Err(UiProjectionError::Invalid("invalid UI region"));
            }
            let id = identifier(values.element(0)?, names)?;
            if !budget.regions.insert(id.clone()) {
                return Err(UiProjectionError::Invalid("duplicate UI region identity"));
            }
            let scroll_target = match values.element(1)?.as_ref().scalar() {
                Ok(SchemeScalar::Boolean(false)) => None,
                _ => {
                    let target = vector(values.element(1)?, 2, 2)?;
                    let id = identifier(target.element(0)?, names)?;
                    let step = match target.element(1)?.as_ref().scalar()? {
                        SchemeScalar::Fixnum(value) if (1..=32).contains(&value) => value as u16,
                        _ => return Err(UiProjectionError::Invalid("invalid UI scroll step")),
                    };
                    Some(UiScrollTarget::from_id(id, step)?)
                }
            };
            let child = Box::new(decode(values.element(2)?, depth + 1, budget, names)?);
            Ok(UiElement::Region {
                id,
                scroll_target,
                child,
            })
        }
        "choice" => {
            if values.len() != 2 {
                return Err(UiProjectionError::Invalid("invalid UI choice"));
            }
            let entries = vector(values.element(0)?, 0, 64)?;
            let mut cases = Vec::with_capacity(entries.len());
            let mut seen = HashSet::new();
            for index in 0..entries.len() {
                let entry = vector(entries.element(index)?, 2, 2)?;
                let fact = identifier(entry.element(0)?, names)?;
                if !seen.insert(fact.clone()) {
                    return Err(UiProjectionError::Invalid("duplicate UI choice fact"));
                }
                cases.push((fact, decode(entry.element(1)?, depth + 1, budget, names)?));
            }
            let fallback = Box::new(decode(values.element(1)?, depth + 1, budget, names)?);
            Ok(UiElement::Choice { cases, fallback })
        }
        "text" => {
            if values.len() > budget.spans {
                return Err(UiProjectionError::Invalid("UI span count exceeds limit"));
            }
            budget.spans -= values.len();
            let mut spans = Vec::with_capacity(values.len());
            for index in 0..values.len() {
                let span = vector(values.element(index)?, 2, 2)?;
                let tone = decode_tone(span.element(0)?, names)?;
                let text = span.element(1)?.into_string()?;
                if text.len().into_result()? > budget.bytes {
                    return Err(UiProjectionError::Invalid("UI text exceeds limit"));
                }
                let text = text.to_string().into_result()?;
                budget.bytes = budget
                    .bytes
                    .checked_sub(text.len())
                    .ok_or(UiProjectionError::Invalid("UI text exceeds limit"))?;
                if text
                    .chars()
                    .any(|character| character.is_control() && character != '\n')
                {
                    return Err(UiProjectionError::Invalid("invalid UI text"));
                }
                spans.push(UiSpan { text, tone });
            }
            Ok(UiElement::Text(spans))
        }
        "rows" | "columns" => {
            let mut children = Vec::with_capacity(values.len());
            for index in 0..values.len() {
                children.push(decode(values.element(index)?, depth + 1, budget, names)?);
            }
            Ok(if kind.as_str() == "rows" {
                UiElement::Rows(children)
            } else {
                UiElement::Columns(children)
            })
        }
        _ => Err(UiProjectionError::Invalid("unknown UI element")),
    }
}

fn inline_template(element: &UiElement) -> bool {
    match element {
        UiElement::Template { .. } => true,
        UiElement::Choice { cases, fallback } => {
            cases.iter().all(|(_, child)| inline_template(child)) && inline_template(fallback)
        }
        _ => false,
    }
}

fn prompt_template(element: &UiElement, field: &UiComponentId) -> bool {
    match element {
        UiElement::Template { spans, .. } => matches!(
            spans.last().map(|span| &span.value),
            Some(UiTemplateValue::Binding(id)) if id == field
        ),
        UiElement::Choice { cases, fallback } => {
            cases.iter().all(|(_, child)| prompt_template(child, field))
                && prompt_template(fallback, field)
        }
        _ => false,
    }
}

pub(crate) fn decode_tone(
    value: RootedSchemeObject<'_>,
    names: &mut IdentifierBuffer,
) -> Result<UiTone, UiProjectionError> {
    match identifier(value, names)?.as_str() {
        "normal" => Ok(UiTone::Normal),
        "canvas" => Ok(UiTone::Canvas),
        "accent" => Ok(UiTone::Accent),
        "info" => Ok(UiTone::Info),
        "success" => Ok(UiTone::Success),
        "warning" => Ok(UiTone::Warning),
        "muted" => Ok(UiTone::Muted),
        _ => Err(UiProjectionError::Invalid("invalid UI span tone")),
    }
}
