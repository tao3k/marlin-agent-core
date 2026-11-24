// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Admit bounded declarative UI trees from rooted Scheme values.

use gerbil_scheme::{IdentifierBuffer, ObjectKind, RootedSchemeObject, SchemeScalar};
use std::collections::HashSet;

use crate::{
    UiCommandName, UiComponentId, UiProjectionError, UiScrollTarget,
    projection::{identifier, vector},
};

use super::{
    UiBackground, UiBarSegment, UiBarSide, UiElement, UiInputAdornment, UiListSource, UiListValue,
    UiSize, UiSlotSource, UiSpan, UiTemplateSpan, UiTemplateValue, UiTone,
};

struct Budget {
    nodes: usize,
    entries: usize,
    bytes: usize,
    regions: HashSet<UiComponentId>,
}

impl Budget {
    fn take_nodes(&mut self, count: usize, error: &'static str) -> Result<(), UiProjectionError> {
        self.nodes = self
            .nodes
            .checked_sub(count)
            .ok_or(UiProjectionError::Invalid(error))?;
        self.entries = self
            .entries
            .checked_sub(count)
            .ok_or(UiProjectionError::Invalid(error))?;
        Ok(())
    }

    fn take_spans(&mut self, count: usize) -> Result<(), UiProjectionError> {
        self.entries = self
            .entries
            .checked_sub(count)
            .ok_or(UiProjectionError::Invalid("UI span count exceeds limit"))?;
        Ok(())
    }
}

fn decode_size(
    kind: RootedSchemeObject<'_>,
    value: RootedSchemeObject<'_>,
    depth: usize,
    budget: &mut Budget,
    names: &mut IdentifierBuffer,
) -> Result<UiSize, UiProjectionError> {
    if depth >= 12 {
        return Err(UiProjectionError::Invalid("UI size tree exceeds limit"));
    }
    budget.take_nodes(1, "UI size tree exceeds limit")?;
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
        // Nodes, size expressions, row data and spans share one entry budget.
        // Keep the tree depth, node count and text bytes independently bounded.
        nodes: 512,
        entries: 768,
        bytes: 32 * 1024,
        regions: HashSet::new(),
    };
    decode(value, 0, &mut budget, &mut IdentifierBuffer::default())
}

fn decode_list_source(
    source: RootedSchemeObject<'_>,
    budget: &mut Budget,
    names: &mut IdentifierBuffer,
) -> Result<UiListSource, UiProjectionError> {
    if source.as_ref().kind()? == ObjectKind::Symbol {
        return match identifier(source, names)?.as_str() {
            "conversation" => Ok(UiListSource::Conversation),
            "messages" => Ok(UiListSource::Messages),
            "workspace" => Ok(UiListSource::Workspace),
            "review" => Ok(UiListSource::Review),
            "candidates" => Ok(UiListSource::Candidates),
            "extensions" => Ok(UiListSource::Extensions),
            _ => Err(UiProjectionError::Invalid("unknown list data capability")),
        };
    }
    let rows = vector(source, 0, 64)?;
    let mut ids = HashSet::new();
    let mut result = Vec::with_capacity(rows.len());
    for index in 0..rows.len() {
        let row = vector(rows.element(index)?, 2, 2)?;
        let key = row.element(0)?;
        let id = if matches!(key.as_ref().scalar(), Ok(SchemeScalar::Boolean(false))) {
            None
        } else {
            let id = identifier(key, names)?;
            if !ids.insert(id.clone()) {
                return Err(UiProjectionError::Invalid("duplicate list row identity"));
            }
            Some(id)
        };
        let fields = vector(row.element(1)?, 0, 16)?;
        budget.take_nodes(fields.len() + 1, "UI list data exceeds node limit")?;
        let mut keys = HashSet::new();
        let mut bindings = Vec::with_capacity(fields.len());
        for entry in 0..fields.len() {
            let field = vector(fields.element(entry)?, 2, 2)?;
            let key = identifier(field.element(0)?, names)?;
            if !keys.insert(key.clone()) {
                return Err(UiProjectionError::Invalid("duplicate list row field"));
            }
            let text = crate::projection::string(field.element(1)?)?;
            budget.bytes = budget
                .bytes
                .checked_sub(key.as_str().len() + text.len())
                .ok_or(UiProjectionError::Invalid(
                    "UI list data exceeds text limit",
                ))?;
            bindings.push((key, text));
        }
        result.push(UiListValue {
            id,
            fields: bindings,
        });
    }
    Ok(UiListSource::Rows(result))
}

fn decode(
    value: RootedSchemeObject<'_>,
    depth: usize,
    budget: &mut Budget,
    names: &mut IdentifierBuffer,
) -> Result<UiElement, UiProjectionError> {
    if depth >= 12 {
        return Err(UiProjectionError::Invalid(
            "UI element nesting exceeds limit",
        ));
    }
    budget.take_nodes(1, "UI element tree exceeds limit")?;
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
                let entry = vector(entries.element(index)?, 5, 6)?;
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
                let command = if entry.len() == 6 {
                    let id = identifier(entry.element(5)?, names)?;
                    if !UiCommandName::is_valid_prefix(id.as_str()) {
                        return Err(UiProjectionError::Invalid("invalid UI bar command"));
                    }
                    Some(id)
                } else {
                    None
                };
                segments.push(UiBarSegment {
                    side,
                    priority,
                    min_width,
                    condition,
                    child: Box::new(child),
                    command,
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
            let border_value = values.element(0)?;
            let border_tone = if matches!(
                border_value.as_ref().scalar(),
                Ok(SchemeScalar::Boolean(false))
            ) {
                None
            } else {
                Some(decode_tone(border_value, names)?)
            };
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
            let source = match identifier(values.element(0)?, names)?.as_str() {
                "selected-buffer" => UiSlotSource::SelectedBuffer,
                "focused-extension" => UiSlotSource::FocusedExtension,
                "runtime-policy" => UiSlotSource::RuntimePolicy,
                "transient-hint" => UiSlotSource::TransientHint,
                _ => return Err(UiProjectionError::Invalid("unknown visual capability")),
            };
            Ok(UiElement::Slot(source))
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
            if !(5..=6).contains(&values.len()) {
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
            let trailing = match (values.len() == 6).then(|| values.element(5)).transpose()? {
                Some(value)
                    if !matches!(value.as_ref().scalar(), Ok(SchemeScalar::Boolean(false))) =>
                {
                    let pair = vector(value, 2, 2)?;
                    let cells = match pair.element(0)?.as_ref().scalar()? {
                        SchemeScalar::Fixnum(cells) if (1..=64).contains(&cells) => cells as u16,
                        _ => {
                            return Err(UiProjectionError::Invalid(
                                "invalid UI input adornment width",
                            ));
                        }
                    };
                    let view = decode(pair.element(1)?, depth + 1, budget, names)?;
                    if !inline_template(&view) {
                        return Err(UiProjectionError::Invalid(
                            "invalid UI input adornment view",
                        ));
                    }
                    Some(UiInputAdornment {
                        cells,
                        view: Box::new(view),
                    })
                }
                _ => None,
            };
            Ok(UiElement::Input {
                bordered,
                background,
                prefix,
                title,
                active,
                trailing,
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
            if values.len() != 4 {
                return Err(UiProjectionError::Invalid("invalid UI list"));
            }
            let id = identifier(values.element(0)?, names)?;
            let source = decode_list_source(values.element(1)?, budget, names)?;
            let size = vector(values.element(2)?, 2, 2)?;
            let offset = decode_size(size.element(0)?, size.element(1)?, 0, budget, names)?;
            let child = Box::new(decode(values.element(3)?, depth + 1, budget, names)?);
            Ok(UiElement::List {
                id,
                source,
                offset,
                child,
            })
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
            budget.take_spans(entries.len())?;
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
            budget.take_spans(values.len())?;
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

pub(crate) fn inline_template(element: &UiElement) -> bool {
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
