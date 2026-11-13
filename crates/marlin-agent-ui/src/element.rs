// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Host-neutral UI primitives admitted from rooted Scheme values.

use gerbil_scheme::{IdentifierBuffer, RootedSchemeObject, SchemeScalar};
use std::collections::HashSet;

use crate::{
    UiComponentId, UiProjectionError,
    projection::{identifier, vector},
};

/// A reusable declarative tree. Product modules compose these primitives.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiElement {
    Text(Vec<UiSpan>),
    /// A styled line whose named fields are bound by the host.
    Template {
        background: UiBackground,
        spans: Vec<UiTemplateSpan>,
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
        scrollable: bool,
        child: Box<UiElement>,
    },
}

/// A fixed, flexible, or host-bound extent on the containing layout axis.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiSize {
    Cells(u16),
    Flex(u16),
    Binding(UiComponentId),
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
    /// Find a named region's contents in a declarative tree.
    pub fn region_child(&self, name: &str) -> Option<&UiElement> {
        match self {
            Self::Region { id, child, .. } if id.as_str() == name => Some(child),
            Self::Region { child, .. } => child.region_child(name),
            Self::Sized { child, .. } => child.region_child(name),
            Self::Rows(children) | Self::Columns(children) => {
                children.iter().find_map(|child| child.region_child(name))
            }
            Self::Choice { cases, fallback } => cases
                .iter()
                .find_map(|(_, child)| child.region_child(name))
                .or_else(|| fallback.region_child(name)),
            Self::Text(_) | Self::Template { .. } => None,
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
            Self::Template { .. } => 1,
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
            Self::Text(_) | Self::Template { .. } => self.intrinsic_rows(),
        }
    }
}

struct Budget {
    nodes: usize,
    spans: usize,
    bytes: usize,
    regions: HashSet<UiComponentId>,
}

/// Project a bounded UI tree through gerbil-scheme's rooted object API.
pub fn decode_element(value: RootedSchemeObject<'_>) -> Result<UiElement, UiProjectionError> {
    let mut budget = Budget {
        nodes: 256,
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
        "sized" => {
            if values.len() != 3 {
                return Err(UiProjectionError::Invalid("invalid UI size"));
            }
            let kind = identifier(values.element(0)?, names)?;
            let size = match kind.as_str() {
                "cells" | "flex" => {
                    let count = match values.element(1)?.as_ref().scalar()? {
                        SchemeScalar::Fixnum(value) => u16::try_from(value)
                            .map_err(|_| UiProjectionError::Invalid("invalid UI size count"))?,
                        _ => return Err(UiProjectionError::Invalid("invalid UI size count")),
                    };
                    if kind.as_str() == "cells" {
                        UiSize::Cells(count)
                    } else {
                        UiSize::Flex(count)
                    }
                }
                "binding" => UiSize::Binding(identifier(values.element(1)?, names)?),
                _ => return Err(UiProjectionError::Invalid("invalid UI size kind")),
            };
            let child = Box::new(decode(values.element(2)?, depth + 1, budget, names)?);
            Ok(UiElement::Sized { size, child })
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
                let entry = vector(entries.element(index)?, 4, 4)?;
                let source = identifier(entry.element(0)?, names)?;
                let tone = decode_tone(entry.element(1)?, names)?;
                let bold = match entry.element(2)?.as_ref().scalar()? {
                    SchemeScalar::Boolean(value) => value,
                    _ => return Err(UiProjectionError::Invalid("invalid UI template weight")),
                };
                let value = match source.as_str() {
                    "literal" => {
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
                    "binding" => UiTemplateValue::Binding(identifier(entry.element(3)?, names)?),
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
            let scrollable = match values.element(1)?.as_ref().scalar()? {
                SchemeScalar::Boolean(value) => value,
                _ => return Err(UiProjectionError::Invalid("invalid UI region scroll flag")),
            };
            let child = Box::new(decode(values.element(2)?, depth + 1, budget, names)?);
            Ok(UiElement::Region {
                id,
                scrollable,
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

fn decode_tone(
    value: RootedSchemeObject<'_>,
    names: &mut IdentifierBuffer,
) -> Result<UiTone, UiProjectionError> {
    match identifier(value, names)?.as_str() {
        "normal" => Ok(UiTone::Normal),
        "accent" => Ok(UiTone::Accent),
        "info" => Ok(UiTone::Info),
        "success" => Ok(UiTone::Success),
        "warning" => Ok(UiTone::Warning),
        "muted" => Ok(UiTone::Muted),
        _ => Err(UiProjectionError::Invalid("invalid UI span tone")),
    }
}
