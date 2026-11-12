// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bounded conversion from Scheme values to owned UI snapshots.

use std::collections::HashSet;

use gerbil_scheme::{
    CollectionKind, IdentifierBuffer, IdentifierKind, ObjectKind, RootedSchemeCollection,
    RootedSchemeObject, SchemeScalar,
};

use crate::model::{
    MAX_COMMANDS, MAX_CONTENT_ITEMS, MAX_ID_BYTES, MAX_REGIONS, MAX_STATUS, MAX_TEXT_CHARS,
    UiCommand, UiCommandName, UiComponentId, UiContent, UiIntent, UiProjectionError, UiRegion,
    UiScrollDirection, UiStatusItem, UiWorkbench,
};

/// Convert a rooted Scheme intent; this does not authorize the action.
pub fn decode_intent(value: RootedSchemeObject<'_>) -> Result<UiIntent, UiProjectionError> {
    let items = vector(value, 1, 2)?;
    let mut names = IdentifierBuffer::default();
    let kind = identifier(items.element(0)?, &mut names)?;
    match (kind.as_str(), items.len()) {
        ("submit-draft", 1) => Ok(UiIntent::SubmitDraft),
        ("focus", 2) => Ok(UiIntent::Focus(identifier(items.element(1)?, &mut names)?)),
        ("scroll", 2) => match identifier(items.element(1)?, &mut names)?.as_str() {
            "up" => Ok(UiIntent::Scroll(UiScrollDirection::Up)),
            "down" => Ok(UiIntent::Scroll(UiScrollDirection::Down)),
            _ => Err(UiProjectionError::Invalid("invalid scroll direction")),
        },
        _ => Err(UiProjectionError::Invalid("invalid UI intent")),
    }
}

/// Validate and copy one Scheme UI projection into independent Rust values.
pub fn decode_workbench(value: RootedSchemeObject<'_>) -> Result<UiWorkbench, UiProjectionError> {
    let root = vector(value, 4, 4)?;
    let model = string(root.element(0)?)?;
    let session = string(root.element(1)?)?;
    let regions = vector(root.element(2)?, 4, MAX_REGIONS)?;
    let mut names = IdentifierBuffer::default();
    let focus = identifier(root.element(3)?, &mut names)?;
    let mut ids = HashSet::new();
    let mut result = Vec::with_capacity(regions.len());
    for index in 0..regions.len() {
        let region = vector(regions.element(index)?, 3, 3)?;
        let id = identifier(region.element(0)?, &mut names)?;
        let kind = identifier(region.element(1)?, &mut names)?;
        if !ids.insert(id.as_str().to_owned()) {
            return Err(UiProjectionError::Invalid("duplicate component identity"));
        }
        let payload = region.element(2)?;
        let node = match kind.as_str() {
            "transcript" if index == 0 && id.as_str() == "transcript" => {
                vector(payload, 0, 0)?;
                UiRegion::Transcript
            }
            "composer" if index == 1 && id.as_str() == "composer" => {
                vector(payload, 0, 0)?;
                UiRegion::Composer
            }
            "commands" if index == 2 && id.as_str() == "commands" => {
                let entries = vector(payload, 0, MAX_COMMANDS)?;
                let mut commands = Vec::with_capacity(entries.len());
                let mut names = HashSet::with_capacity(entries.len());
                for i in 0..entries.len() {
                    let row = vector(entries.element(i)?, 3, 3)?;
                    let name = UiCommandName::new(&string(row.element(0)?)?)?;
                    if !names.insert(name.clone()) {
                        return Err(UiProjectionError::Invalid("duplicate command name"));
                    }
                    let description = string(row.element(1)?)?;
                    let accepts_arguments = match row.element(2)?.as_ref().scalar()? {
                        SchemeScalar::Boolean(value) => value,
                        _ => {
                            return Err(UiProjectionError::Invalid(
                                "invalid command arguments flag",
                            ));
                        }
                    };
                    commands.push(UiCommand {
                        name,
                        description,
                        accepts_arguments,
                    });
                }
                UiRegion::Commands(commands)
            }
            "status" if index + 1 == regions.len() && id.as_str() == "status" => {
                let children = vector(payload, 0, MAX_STATUS)?;
                let mut items = Vec::with_capacity(children.len());
                for item_index in 0..children.len() {
                    let item = vector(children.element(item_index)?, 3, 3)?;
                    let id = identifier(item.element(0)?, &mut names)?;
                    if identifier(item.element(1)?, &mut names)?.as_str() != "status-item"
                        || !ids.insert(id.as_str().to_owned())
                    {
                        return Err(UiProjectionError::Invalid(
                            "invalid status identity or kind",
                        ));
                    }
                    items.push(UiStatusItem {
                        id,
                        label: string(item.element(2)?)?,
                    });
                }
                UiRegion::Status(items)
            }
            "extension" if index >= 3 && index + 1 < regions.len() => {
                let data = vector(payload, 3, 3)?;
                let label = string(data.element(0)?)?;
                let focusable = match data.element(1)?.as_ref().scalar()? {
                    SchemeScalar::Boolean(value) => value,
                    _ => return Err(UiProjectionError::Invalid("invalid focusable value")),
                };
                let entries = vector(data.element(2)?, 0, MAX_CONTENT_ITEMS)?;
                let mut content = Vec::with_capacity(entries.len());
                for item_index in 0..entries.len() {
                    let item = vector(entries.element(item_index)?, 2, 3)?;
                    let kind = identifier(item.element(0)?, &mut names)?;
                    let value = match (kind.as_str(), item.len()) {
                        ("text", 2) => UiContent::Text(string(item.element(1)?)?),
                        ("field", 3) => UiContent::Field {
                            label: string(item.element(1)?)?,
                            value: string(item.element(2)?)?,
                        },
                        _ => return Err(UiProjectionError::Invalid("invalid UI content item")),
                    };
                    content.push(value);
                }
                UiRegion::Extension {
                    id,
                    label,
                    focusable,
                    content,
                }
            }
            _ => return Err(UiProjectionError::Invalid("invalid workbench region")),
        };
        result.push(node);
    }
    if !matches!(result.last(), Some(UiRegion::Status(_))) {
        return Err(UiProjectionError::Invalid("missing status region"));
    }
    let focus_valid = result.iter().any(|region| match region {
        UiRegion::Transcript => focus.as_str() == "transcript",
        UiRegion::Composer => focus.as_str() == "composer",
        UiRegion::Commands(_) => focus.as_str() == "commands",
        UiRegion::Extension {
            id,
            focusable: true,
            ..
        } => *id == focus,
        _ => false,
    });
    if !focus_valid {
        return Err(UiProjectionError::Invalid("invalid focused component"));
    }
    Ok(UiWorkbench {
        model_label: model,
        session_label: session,
        regions: result,
        focus,
    })
}

pub(crate) fn vector(
    value: RootedSchemeObject<'_>,
    min: usize,
    max: usize,
) -> Result<RootedSchemeCollection<'_>, UiProjectionError> {
    let value = value.into_collection(CollectionKind::Vector)?;
    if !(min..=max).contains(&value.len()) {
        return Err(UiProjectionError::Invalid("invalid vector size"));
    }
    Ok(value)
}

pub(crate) fn string(value: RootedSchemeObject<'_>) -> Result<String, UiProjectionError> {
    let value = value.into_string()?;
    if value.len().into_result()? > MAX_TEXT_CHARS {
        return Err(UiProjectionError::Invalid("text exceeds UI limit"));
    }
    Ok(value.to_string().into_result()?)
}

fn identifier(
    value: RootedSchemeObject<'_>,
    buffer: &mut IdentifierBuffer,
) -> Result<UiComponentId, UiProjectionError> {
    if value.as_ref().kind()? != ObjectKind::Symbol {
        return Err(UiProjectionError::Invalid("expected symbol"));
    }
    value.as_ref().copy_identifier_names(buffer)?;
    let Some((IdentifierKind::Symbol, name)) = buffer.get(0) else {
        return Err(UiProjectionError::Invalid("invalid symbol"));
    };
    if buffer.len() != 1 || name.is_empty() || name.len() > MAX_ID_BYTES {
        return Err(UiProjectionError::Invalid("invalid identity length"));
    }
    UiComponentId::new(name)
}
