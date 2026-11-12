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
    UiCommand, UiCommandName, UiComponentId, UiContent, UiIntent, UiLayoutPolicy,
    UiProjectionError, UiRegion, UiRgb, UiScrollDirection, UiStatusItem, UiStatusTone, UiTheme,
    UiWorkbench,
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
    let root = vector(value, 6, 6)?;
    let model = string(root.element(0)?)?;
    let session = string(root.element(1)?)?;
    let mut names = IdentifierBuffer::default();
    let focus = identifier(root.element(3)?, &mut names)?;
    let layout = decode_layout(root.element(4)?)?;
    let theme = decode_theme(root.element(5)?)?;
    let regions = decode_regions(root.element(2)?, &focus)?;
    Ok(UiWorkbench {
        model_label: model,
        session_label: session,
        regions,
        layout,
        theme,
        focus,
    })
}

fn decode_layout(value: RootedSchemeObject<'_>) -> Result<UiLayoutPolicy, UiProjectionError> {
    let layout_values = vector(value, 3, 3)?;
    let field = |index, lower, upper| -> Result<u16, UiProjectionError> {
        match layout_values.element(index)?.as_ref().scalar()? {
            SchemeScalar::Fixnum(value) if (lower..=upper).contains(&value) => Ok(value as u16),
            _ => Err(UiProjectionError::Invalid("invalid UI layout policy")),
        }
    };
    let layout = UiLayoutPolicy {
        min_transcript_rows: field(0, 1, 4)?,
        max_chatbox_rows: field(1, 3, 20)?,
        max_command_rows: field(2, 1, 8)?,
    };
    Ok(layout)
}

fn decode_theme(value: RootedSchemeObject<'_>) -> Result<UiTheme, UiProjectionError> {
    let theme_values = vector(value, 8, 8)?;
    let color = |index| -> Result<UiRgb, UiProjectionError> {
        let channels = vector(theme_values.element(index)?, 3, 3)?;
        let channel = |position| -> Result<u8, UiProjectionError> {
            match channels.element(position)?.as_ref().scalar()? {
                SchemeScalar::Fixnum(value) if (0..=255).contains(&value) => Ok(value as u8),
                _ => Err(UiProjectionError::Invalid("invalid UI color channel")),
            }
        };
        Ok(UiRgb::new(channel(0)?, channel(1)?, channel(2)?))
    };
    let theme = UiTheme {
        canvas: color(0)?,
        ink: color(1)?,
        bar: color(2)?,
        accent: color(3)?,
        info: color(4)?,
        success: color(5)?,
        warning: color(6)?,
        muted: color(7)?,
    };
    Ok(theme)
}

fn decode_regions(
    value: RootedSchemeObject<'_>,
    focus: &UiComponentId,
) -> Result<Vec<UiRegion>, UiProjectionError> {
    let regions = vector(value, 4, MAX_REGIONS)?;
    let mut names = IdentifierBuffer::default();
    let mut ids = HashSet::new();
    let mut modeline_slots = [false; 8];
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
            "commands" if index == 2 && id.as_str() == "commands" => decode_commands(payload)?,
            "status" if index + 1 == regions.len() && id.as_str() == "status" => {
                decode_status(payload, &mut ids, &mut names, &mut modeline_slots)?
            }
            "extension" if index >= 3 && index + 1 < regions.len() => {
                decode_extension(id, payload, &mut names)?
            }
            _ => return Err(UiProjectionError::Invalid("invalid workbench region")),
        };
        result.push(node);
    }
    if !matches!(result.last(), Some(UiRegion::Status(_))) {
        return Err(UiProjectionError::Invalid("missing status region"));
    }
    if modeline_slots.iter().any(|slot| *slot) && !modeline_slots.iter().all(|slot| *slot) {
        return Err(UiProjectionError::Invalid("incomplete modeline"));
    }
    let focus_valid = result.iter().any(|region| match region {
        UiRegion::Transcript => focus.as_str() == "transcript",
        UiRegion::Composer => focus.as_str() == "composer",
        UiRegion::Commands(_) => focus.as_str() == "commands",
        UiRegion::Extension {
            id,
            focusable: true,
            ..
        } => id == focus,
        _ => false,
    });
    if !focus_valid {
        return Err(UiProjectionError::Invalid("invalid focused component"));
    }
    Ok(result)
}

fn decode_commands(payload: RootedSchemeObject<'_>) -> Result<UiRegion, UiProjectionError> {
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
                return Err(UiProjectionError::Invalid("invalid command arguments flag"));
            }
        };
        commands.push(UiCommand {
            name,
            description,
            accepts_arguments,
        });
    }
    Ok(UiRegion::Commands(commands))
}

fn decode_status(
    payload: RootedSchemeObject<'_>,
    ids: &mut HashSet<String>,
    names: &mut IdentifierBuffer,
    modeline_slots: &mut [bool; 8],
) -> Result<UiRegion, UiProjectionError> {
    let children = vector(payload, 0, MAX_STATUS)?;
    let mut items = Vec::with_capacity(children.len());
    for item_index in 0..children.len() {
        let item = vector(children.element(item_index)?, 6, 6)?;
        let id = identifier(item.element(0)?, names)?;
        if let Some(index) = match id.as_str() {
            "modeline/left/brand" => Some(0),
            "modeline/left/model" => Some(1),
            "modeline/left/activity" => Some(2),
            "modeline/right/turns" => Some(3),
            "modeline/right/session" => Some(4),
            "modeline/state/online" => Some(5),
            "modeline/state/offline" => Some(6),
            "modeline/state/unknown" => Some(7),
            _ => None,
        } {
            if item_index != index {
                return Err(UiProjectionError::Invalid("modeline order changed"));
            }
            modeline_slots[index] = true;
        } else if id.as_str().starts_with("modeline/") {
            return Err(UiProjectionError::Invalid("unknown modeline slot"));
        }
        if identifier(item.element(1)?, names)?.as_str() != "status-item"
            || !ids.insert(id.as_str().to_owned())
        {
            return Err(UiProjectionError::Invalid(
                "invalid status identity or kind",
            ));
        }
        let label = string(item.element(2)?)?;
        let compact_label = string(item.element(3)?)?;
        let tone = match identifier(item.element(4)?, names)?.as_str() {
            "accent" => UiStatusTone::Accent,
            "info" => UiStatusTone::Info,
            "success" => UiStatusTone::Success,
            "warning" => UiStatusTone::Warning,
            "muted" => UiStatusTone::Muted,
            _ => return Err(UiProjectionError::Invalid("invalid status tone")),
        };
        let active_tone = match identifier(item.element(5)?, names)?.as_str() {
            "accent" => UiStatusTone::Accent,
            "info" => UiStatusTone::Info,
            "success" => UiStatusTone::Success,
            "warning" => UiStatusTone::Warning,
            "muted" => UiStatusTone::Muted,
            _ => return Err(UiProjectionError::Invalid("invalid active status tone")),
        };
        if id.as_str().starts_with("modeline/") {
            let dynamic = matches!(
                id.as_str(),
                "modeline/left/model" | "modeline/right/turns" | "modeline/right/session"
            );
            if (dynamic && (!label.is_empty() || !compact_label.is_empty()))
                || (!dynamic && (label.is_empty() || compact_label.is_empty()))
            {
                return Err(UiProjectionError::Invalid("invalid modeline labels"));
            }
        } else if compact_label != label
            || tone != UiStatusTone::Muted
            || active_tone != UiStatusTone::Muted
        {
            return Err(UiProjectionError::Invalid(
                "invalid host status presentation",
            ));
        }
        items.push(UiStatusItem {
            id,
            label,
            compact_label,
            tone,
            active_tone,
        });
    }
    Ok(UiRegion::Status(items))
}

fn decode_extension(
    id: UiComponentId,
    payload: RootedSchemeObject<'_>,
    names: &mut IdentifierBuffer,
) -> Result<UiRegion, UiProjectionError> {
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
        let kind = identifier(item.element(0)?, names)?;
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
    Ok(UiRegion::Extension {
        id,
        label,
        focusable,
        content,
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
