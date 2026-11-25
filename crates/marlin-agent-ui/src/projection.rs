// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bounded conversion from Scheme values to owned UI snapshots.

use std::collections::HashSet;

use gerbil_scheme::{
    CollectionKind, IdentifierBuffer, IdentifierKind, ObjectKind, RootedSchemeCollection,
    RootedSchemeObject, SchemeScalar,
};
use marlin_agent_protocol::HumanDecision;

use crate::model::{
    MAX_COMMANDS, MAX_ID_BYTES, MAX_REGIONS, MAX_STATUS, MAX_TEXT_CHARS, MODELINE_SLOTS,
    UiBufferId, UiCommand, UiCommandArguments, UiCommandCapability, UiCommandName, UiComponentId,
    UiFrame, UiFrameSpec, UiIntent, UiKeyBinding, UiKeyCommand, UiKeyMode, UiKeyScope, UiKeyToken,
    UiLayoutPolicy, UiProjectionError, UiRegion, UiReviewChoice, UiReviewMenu, UiRgb,
    UiScrollDirection, UiStatusItem, UiStatusTone, UiTheme, UiToolReviewChoice,
    UiToolReviewDecision, UiToolReviewMenu, UiTransient, UiTransientForeign, UiTransientHead,
    UiTransientId, UiWindow, UiWorkbench,
};

/// Admit a finite Scheme presentation without granting review authority.
pub fn decode_review_menu(
    value: RootedSchemeObject<'_>,
) -> Result<UiReviewMenu, UiProjectionError> {
    let menu = vector(value, 2, 2)?;
    let prompt = string(menu.element(0)?)?;
    if prompt.is_empty() {
        return Err(UiProjectionError::Invalid("review prompt is empty"));
    }
    let entries = vector(menu.element(1)?, 4, 4)?;
    let mut symbols = IdentifierBuffer::default();
    let mut seen = HashSet::with_capacity(entries.len());
    let mut choices = Vec::with_capacity(entries.len());
    for index in 0..entries.len() {
        let entry = vector(entries.element(index)?, 3, 3)?;
        let decision = match identifier(entry.element(0)?, &mut symbols)?.as_str() {
            "approved" => HumanDecision::Approved,
            "rejected" => HumanDecision::Rejected,
            "needs-question" => HumanDecision::NeedsQuestion,
            "patch-requested" => HumanDecision::PatchRequested,
            _ => return Err(UiProjectionError::Invalid("unknown human review decision")),
        };
        if !seen.insert(std::mem::discriminant(&decision)) {
            return Err(UiProjectionError::Invalid(
                "duplicate human review decision",
            ));
        }
        let label = string(entry.element(1)?)?;
        let description = string(entry.element(2)?)?;
        if label.is_empty() || description.is_empty() {
            return Err(UiProjectionError::Invalid("empty human review choice"));
        }
        choices.push(UiReviewChoice {
            decision,
            label,
            description,
        });
    }
    Ok(UiReviewMenu { prompt, choices })
}

/// Admit only the two finite tool-review intents. Neither intent is authority
/// to execute a tool; the runtime must validate the durable request.
pub fn decode_tool_review_menu(
    value: RootedSchemeObject<'_>,
) -> Result<UiToolReviewMenu, UiProjectionError> {
    let menu = vector(value, 3, 3)?;
    let prompt = string(menu.element(0)?)?;
    if prompt.is_empty() {
        return Err(UiProjectionError::Invalid("tool review prompt is empty"));
    }
    let details_heading = string(menu.element(2)?)?;
    if details_heading.is_empty() {
        return Err(UiProjectionError::Invalid("tool review heading is empty"));
    }
    let entries = vector(menu.element(1)?, 2, 2)?;
    let mut symbols = IdentifierBuffer::default();
    let mut seen = HashSet::with_capacity(entries.len());
    let mut choices = Vec::with_capacity(entries.len());
    for index in 0..entries.len() {
        let entry = vector(entries.element(index)?, 3, 3)?;
        let decision = match identifier(entry.element(0)?, &mut symbols)?.as_str() {
            "approve-once" => UiToolReviewDecision::ApproveOnce,
            "reject" => UiToolReviewDecision::Reject,
            _ => return Err(UiProjectionError::Invalid("unknown tool review decision")),
        };
        if !seen.insert(decision) {
            return Err(UiProjectionError::Invalid("duplicate tool review decision"));
        }
        let label = string(entry.element(1)?)?;
        let description = string(entry.element(2)?)?;
        if label.is_empty() || description.is_empty() {
            return Err(UiProjectionError::Invalid("empty tool review choice"));
        }
        choices.push(UiToolReviewChoice {
            decision,
            label,
            description,
        });
    }
    Ok(UiToolReviewMenu {
        prompt,
        choices,
        details_heading,
    })
}

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
        ("select-buffer", 2) => Ok(UiIntent::SelectBuffer(UiBufferId::new(
            identifier(items.element(1)?, &mut names)?.as_str(),
        )?)),
        _ => Err(UiProjectionError::Invalid("invalid UI intent")),
    }
}

/// Validate and copy one Scheme UI projection into independent Rust values.
pub fn decode_workbench(value: RootedSchemeObject<'_>) -> Result<UiWorkbench, UiProjectionError> {
    let root = vector(value, 9, 9).map_err(|error| error.at("workbench"))?;
    let model = string(root.element(0)?).map_err(|error| error.at("model"))?;
    let session = string(root.element(1)?).map_err(|error| error.at("session"))?;
    let mut names = IdentifierBuffer::default();
    let focus = identifier(root.element(3)?, &mut names).map_err(|error| error.at("focus"))?;
    let layout = decode_layout(root.element(4)?).map_err(|error| error.at("layout"))?;
    let theme = decode_theme(root.element(5)?).map_err(|error| error.at("theme"))?;
    let frame = decode_frame_state(root.element(6)?).map_err(|error| error.at("frame"))?;
    let keymap = decode_keymap(root.element(7)?).map_err(|error| error.at("keymap"))?;
    let transients = decode_transients(root.element(8)?).map_err(|error| error.at("transients"))?;
    let regions = decode_regions(root.element(2)?, &focus).map_err(|error| error.at("regions"))?;
    Ok(UiWorkbench {
        model_label: model,
        session_label: session,
        regions,
        layout,
        theme,
        focus,
        frame,
        keymap,
        transients,
    })
}

fn decode_keymap(value: RootedSchemeObject<'_>) -> Result<Vec<UiKeyBinding>, UiProjectionError> {
    let entries = vector(value, 0, 128)?;
    let mut names = IdentifierBuffer::default();
    let mut result = Vec::with_capacity(entries.len());
    for index in 0..entries.len() {
        let entry = vector(entries.element(index)?, 3, 3)?;
        let scope = match identifier(entry.element(0)?, &mut names)?.as_str() {
            "global" => UiKeyScope::Global,
            "insert" => UiKeyScope::Mode(UiKeyMode::Insert),
            "normal" => UiKeyScope::Mode(UiKeyMode::Normal),
            _ => return Err(UiProjectionError::Invalid("invalid UI key scope")),
        };
        let keys = vector(entry.element(1)?, 1, 4)?;
        let mut sequence = Vec::with_capacity(keys.len());
        for position in 0..keys.len() {
            sequence.push(UiKeyToken::new(
                identifier(keys.element(position)?, &mut names)?.as_str(),
            )?);
        }
        let command = decode_key_command(entry.element(2)?, &mut names)?;
        result.push(UiKeyBinding {
            scope,
            sequence,
            command,
        });
    }
    let continuations: HashSet<_> = result
        .iter()
        .filter(|binding| !matches!(binding.command, UiKeyCommand::PrefixHint { .. }))
        .flat_map(|binding| {
            (1..binding.sequence.len())
                .map(move |length| (binding.scope, &binding.sequence[..length]))
        })
        .collect();
    for (index, binding) in result.iter().enumerate() {
        let prefix = binding.sequence.as_slice();
        if matches!(binding.command, UiKeyCommand::PrefixHint { .. })
            && !continuations.contains(&(binding.scope, prefix))
            && !continuations.contains(&(UiKeyScope::Global, prefix))
        {
            return Err(UiProjectionError::Invalid("key hint has no continuation"));
        }
        for other in &result[index + 1..] {
            let overlap = binding.scope == UiKeyScope::Global
                || other.scope == UiKeyScope::Global
                || binding.scope == other.scope;
            if overlap
                && (binding.sequence.starts_with(&other.sequence)
                    || other.sequence.starts_with(&binding.sequence))
                && (binding.sequence == other.sequence
                    || (!matches!(binding.command, UiKeyCommand::PrefixHint { .. })
                        && !matches!(other.command, UiKeyCommand::PrefixHint { .. })))
            {
                return Err(UiProjectionError::Invalid("ambiguous UI key prefix"));
            }
        }
    }
    Ok(result)
}

fn is_passive_hint(element: &crate::UiElement) -> bool {
    use crate::UiElement;
    match element {
        UiElement::Text(_) => true,
        UiElement::Rows(children) | UiElement::Columns(children) => {
            children.iter().all(is_passive_hint)
        }
        UiElement::Sized { child, .. } => is_passive_hint(child),
        UiElement::Choice { cases, fallback } => {
            cases.iter().all(|(_, child)| is_passive_hint(child)) && is_passive_hint(fallback)
        }
        _ => false,
    }
}

fn decode_key_command(
    value: RootedSchemeObject<'_>,
    names: &mut IdentifierBuffer,
) -> Result<UiKeyCommand, UiProjectionError> {
    use crate::model::UiEditorAction;

    if value.as_ref().kind()? != ObjectKind::Symbol {
        let action = vector(value, 2, 4)?;
        let name = identifier(action.element(0)?, names)?;
        if name.as_str() == "prefix-hint" && (2..=3).contains(&action.len()) {
            let hint = crate::decode_element(action.element(1)?)?;
            if !is_passive_hint(&hint) || hint.intrinsic_rows() > 8 {
                return Err(UiProjectionError::Invalid("invalid key prefix hint"));
            }
            let milliseconds = if action.len() == 3 {
                match action.element(2)?.as_ref().scalar()? {
                    SchemeScalar::Fixnum(value) if (0..=5000).contains(&value) => value as u64,
                    _ => return Err(UiProjectionError::Invalid("invalid key prefix hint delay")),
                }
            } else {
                0
            };
            return Ok(UiKeyCommand::PrefixHint {
                view: Box::new(hint),
                delay: std::time::Duration::from_millis(milliseconds),
            });
        }
        if name.as_str() == "read-selection-character" && action.len() == 3 {
            let inclusive = match identifier(action.element(1)?, names)?.as_str() {
                "through" => true,
                "before" => false,
                _ => {
                    return Err(UiProjectionError::Invalid(
                        "invalid character selection extent",
                    ));
                }
            };
            let backward = match identifier(action.element(2)?, names)?.as_str() {
                "forward" => false,
                "backward" => true,
                _ => {
                    return Err(UiProjectionError::Invalid(
                        "invalid character selection direction",
                    ));
                }
            };
            return Ok(UiKeyCommand::ReadCharacter(crate::UiCharacterSelection {
                inclusive,
                backward,
            }));
        }
        if action.len() != 4 {
            return Err(UiProjectionError::Invalid(
                "invalid parameterized key action",
            ));
        }
        if name.as_str() == "expand-selection" {
            let steps = match action.element(1)?.as_ref().scalar()? {
                SchemeScalar::Fixnum(steps) => {
                    u8::try_from(steps).ok().and_then(std::num::NonZeroU8::new)
                }
                _ => None,
            }
            .ok_or(UiProjectionError::Invalid(
                "invalid selection expansion count",
            ))?;
            let at_rest_start = match action.element(3)?.as_ref().scalar()? {
                SchemeScalar::Boolean(value) => value,
                _ => {
                    return Err(UiProjectionError::Invalid(
                        "invalid selection expansion fallback",
                    ));
                }
            };
            return Ok(UiKeyCommand::Edit(UiEditorAction::ExpandSelection {
                steps,
                tone: crate::element::decode_tone(action.element(2)?, names)?,
                at_rest_start,
            }));
        }
        if name.as_str() != "select-delimited" {
            return Err(UiProjectionError::Invalid(
                "invalid parameterized editor action",
            ));
        }
        let inclusive = match identifier(action.element(1)?, names)?.as_str() {
            "inner" => false,
            "bounds" => true,
            _ => return Err(UiProjectionError::Invalid("invalid selection extent")),
        };
        let delimiter = |index| match action.element(index)?.as_ref().scalar()? {
            SchemeScalar::Char(value) if value.is_ascii_graphic() && value != '\\' => Ok(value),
            _ => Err(UiProjectionError::Invalid("invalid selection delimiter")),
        };
        return Ok(UiKeyCommand::Edit(UiEditorAction::SelectDelimited {
            open: delimiter(2)?,
            close: delimiter(3)?,
            inclusive,
        }));
    }
    let kind = identifier(value, names)?;
    Ok(match kind.as_str() {
        "open-commands" => UiKeyCommand::OpenCommands,
        "cancel" => UiKeyCommand::Cancel,
        "normal-mode" => UiKeyCommand::Mode(UiKeyMode::Normal),
        "insert-mode" => UiKeyCommand::Mode(UiKeyMode::Insert),
        "append-insert" => UiKeyCommand::Edit(UiEditorAction::AppendInsert),
        "open-line-above" => UiKeyCommand::Edit(UiEditorAction::OpenLine { below: false }),
        "open-line-below" => UiKeyCommand::Edit(UiEditorAction::OpenLine { below: true }),
        "previous-line" => UiKeyCommand::Edit(UiEditorAction::PreviousLine),
        "next-line" => UiKeyCommand::Edit(UiEditorAction::NextLine),
        "next-word" => UiKeyCommand::Edit(UiEditorAction::NextWord),
        "previous-word" => UiKeyCommand::Edit(UiEditorAction::PreviousWord),
        "delete-line" => UiKeyCommand::Edit(UiEditorAction::DeleteLine),
        "undo" => UiKeyCommand::Edit(UiEditorAction::Undo),
        "history-search" => UiKeyCommand::Edit(UiEditorAction::HistorySearch),
        "previous-prompt" => UiKeyCommand::Edit(UiEditorAction::PreviousPrompt),
        "next-prompt" => UiKeyCommand::Edit(UiEditorAction::NextPrompt),
        "insert-newline" => UiKeyCommand::Edit(UiEditorAction::InsertNewline),
        "backward-char" => UiKeyCommand::Edit(UiEditorAction::BackwardChar),
        "forward-char" => UiKeyCommand::Edit(UiEditorAction::ForwardChar),
        "delete-char" => UiKeyCommand::Edit(UiEditorAction::DeleteChar),
        "line-start" => UiKeyCommand::Edit(UiEditorAction::LineStart),
        "line-end" => UiKeyCommand::Edit(UiEditorAction::LineEnd),
        "select-word" => UiKeyCommand::Edit(UiEditorAction::SelectWord),
        "select-previous-word" => UiKeyCommand::Edit(UiEditorAction::SelectPreviousWord),
        "select-next-word" => UiKeyCommand::Edit(UiEditorAction::SelectNextWord),
        "select-line" => UiKeyCommand::Edit(UiEditorAction::SelectLine),
        "extend-left" => UiKeyCommand::Edit(UiEditorAction::ExtendLeft),
        "extend-right" => UiKeyCommand::Edit(UiEditorAction::ExtendRight),
        "extend-up" => UiKeyCommand::Edit(UiEditorAction::ExtendUp),
        "extend-down" => UiKeyCommand::Edit(UiEditorAction::ExtendDown),
        "reverse-selection" => UiKeyCommand::Edit(UiEditorAction::ReverseSelection),
        "previous-selection" => UiKeyCommand::Edit(UiEditorAction::PreviousSelection),
        "clear-selection" => UiKeyCommand::Edit(UiEditorAction::ClearSelection),
        "delete-backward" => UiKeyCommand::Edit(UiEditorAction::DeleteBackward),
        "cut-selection" => UiKeyCommand::Edit(UiEditorAction::CutSelection),
        "change-selection" => UiKeyCommand::Edit(UiEditorAction::ChangeSelection),
        "copy-selection" => UiKeyCommand::Edit(UiEditorAction::CopySelection),
        "paste-selection" => UiKeyCommand::Edit(UiEditorAction::PasteSelection),
        name => UiKeyCommand::Invoke(UiCommandName::new(name)?),
    })
}

fn decode_transients(value: RootedSchemeObject<'_>) -> Result<Vec<UiTransient>, UiProjectionError> {
    let entries = vector(value, 0, 8)?;
    let mut names = IdentifierBuffer::default();
    let mut seen = HashSet::with_capacity(entries.len());
    let mut result = Vec::with_capacity(entries.len());
    for index in 0..entries.len() {
        let entry = vector(entries.element(index)?, 5, 5)?;
        let id = UiTransientId::new(identifier(entry.element(0)?, &mut names)?.as_str())?;
        if !seen.insert(id.clone()) {
            return Err(UiProjectionError::Invalid("duplicate transient identity"));
        }
        result.push(decode_transient(&entry, &mut names, id)?);
    }
    Ok(result)
}

fn decode_transient(
    entry: &RootedSchemeCollection<'_>,
    names: &mut IdentifierBuffer,
    id: UiTransientId,
) -> Result<UiTransient, UiProjectionError> {
    let hint = crate::decode_element(entry.element(1)?)?;
    let warning_hint = crate::decode_element(entry.element(4)?)?;
    if hint.intrinsic_rows() > 8 || warning_hint.intrinsic_rows() > 8 {
        return Err(UiProjectionError::Invalid(
            "transient hint exceeds row limit",
        ));
    }
    let foreign = match identifier(entry.element(2)?, names)?.as_str() {
        "exit" => UiTransientForeign::Exit,
        "warn" => UiTransientForeign::Warn,
        "run" => UiTransientForeign::Run,
        _ => {
            return Err(UiProjectionError::Invalid(
                "invalid transient foreign key policy",
            ));
        }
    };
    let values = vector(entry.element(3)?, 1, 16)?;
    let mut seen = HashSet::with_capacity(values.len());
    let mut heads = Vec::with_capacity(values.len());
    for index in 0..values.len() {
        let head = vector(values.element(index)?, 4, 4)?;
        let key = UiKeyToken::new(identifier(head.element(0)?, names)?.as_str())?;
        if !seen.insert(key.clone()) {
            return Err(UiProjectionError::Invalid("duplicate transient head"));
        }
        let command = match identifier(head.element(1)?, names)?.as_str() {
            "exit" => None,
            name => Some(UiCommandName::new(name)?),
        };
        let label = string(head.element(2)?)?;
        let exit = match head.element(3)?.as_ref().scalar()? {
            SchemeScalar::Boolean(value) => value,
            _ => return Err(UiProjectionError::Invalid("invalid transient exit flag")),
        };
        if label.len() > 64 || label.chars().any(char::is_control) || (command.is_none() && !exit) {
            return Err(UiProjectionError::Invalid("invalid transient head"));
        }
        heads.push(UiTransientHead {
            key,
            command,
            label,
            exit,
        });
    }
    Ok(UiTransient {
        id,
        hint,
        warning_hint,
        foreign,
        heads,
    })
}

fn decode_frame_state(value: RootedSchemeObject<'_>) -> Result<UiFrame, UiProjectionError> {
    let fields = vector(value, 5, 5)?;
    let mut names = IdentifierBuffer::default();
    let id = identifier(fields.element(0)?, &mut names)?;
    let selected_window = identifier(fields.element(2)?, &mut names)?;
    let minibuffer = identifier(fields.element(3)?, &mut names)?;
    let catalog = vector(fields.element(4)?, 1, 16)?;
    let mut buffers = Vec::with_capacity(catalog.len());
    let mut buffer_names = HashSet::new();
    for index in 0..catalog.len() {
        let name = UiBufferId::new(identifier(catalog.element(index)?, &mut names)?.as_str())?;
        if !buffer_names.insert(name.as_str().to_owned()) {
            return Err(UiProjectionError::Invalid("duplicate UI buffer"));
        }
        buffers.push(name);
    }
    let projected_windows = vector(fields.element(1)?, 1, 8)?;
    let mut windows = Vec::with_capacity(projected_windows.len());
    let mut window_names = HashSet::new();
    for index in 0..projected_windows.len() {
        let pair = vector(projected_windows.element(index)?, 2, 2)?;
        let window_id = identifier(pair.element(0)?, &mut names)?;
        let buffer = UiBufferId::new(identifier(pair.element(1)?, &mut names)?.as_str())?;
        if !window_names.insert(window_id.as_str().to_owned())
            || !buffer_names.contains(buffer.as_str())
        {
            return Err(UiProjectionError::Invalid("invalid UI window binding"));
        }
        windows.push(UiWindow {
            id: window_id,
            buffer,
        });
    }
    let frame = UiFrame {
        id,
        windows,
        selected_window,
        minibuffer,
        buffers,
    };
    if frame.selected_buffer().is_none() {
        return Err(UiProjectionError::Invalid("missing selected UI window"));
    }
    Ok(frame)
}

fn decode_layout(value: RootedSchemeObject<'_>) -> Result<UiLayoutPolicy, UiProjectionError> {
    let layout_values = vector(value, 8, 8)?;
    let field = |index, lower, upper| -> Result<u16, UiProjectionError> {
        match layout_values.element(index)?.as_ref().scalar()? {
            SchemeScalar::Fixnum(value) if (lower..=upper).contains(&value) => Ok(value as u16),
            _ => Err(UiProjectionError::Invalid("invalid UI layout policy")),
        }
    };
    let fold = vector(layout_values.element(4)?, 5, 5)?;
    let default_collapsed = match fold.element(0)?.as_ref().scalar()? {
        SchemeScalar::Boolean(value) => value,
        _ => return Err(UiProjectionError::Invalid("invalid thinking fold flag")),
    };
    let label = |index, allow_empty: bool| -> Result<String, UiProjectionError> {
        let value = string(fold.element(index)?)?;
        if (!allow_empty && value.is_empty())
            || value.len() > 32
            || value.chars().any(char::is_control)
        {
            return Err(UiProjectionError::Invalid("invalid transcript label"));
        }
        Ok(value)
    };
    let frame_tree = crate::decode_element(layout_values.element(5)?)?;
    let layout = UiLayoutPolicy {
        message_frame: decode_frame(layout_values.element(0)?)?,
        command_frame: decode_frame(layout_values.element(1)?)?,
        max_chatbox_rows: field(2, 3, 20)?,
        max_command_rows: field(3, 1, MAX_COMMANDS as isize)?,
        frame_tree,
        regions: decode_frame_regions(layout_values.element(6)?)?,
        echo: decode_echo_policy(layout_values.element(7)?)?,
        transcript_policy: super::model::UiTranscriptPolicy {
            default_collapsed,
            user_prefix: label(1, false)?,
            assistant_role: label(2, true)?,
            thinking_collapsed_role: label(3, false)?,
            thinking_expanded_role: label(4, false)?,
        },
    };
    Ok(layout)
}

fn decode_echo_policy(
    value: RootedSchemeObject<'_>,
) -> Result<crate::UiEchoPolicy, UiProjectionError> {
    let fields = vector(value, 3, 3)?;
    let view = crate::decode_element(fields.element(0)?)?;
    if !crate::element::inline_template(&view) {
        return Err(UiProjectionError::Invalid("invalid echo area template"));
    }
    let timeout = match fields.element(1)?.as_ref().scalar()? {
        SchemeScalar::Fixnum(value) if (100..=30_000).contains(&value) => value as u64,
        _ => return Err(UiProjectionError::Invalid("invalid echo area timeout")),
    };
    let clear_on_input = match fields.element(2)?.as_ref().scalar()? {
        SchemeScalar::Boolean(value) => value,
        _ => return Err(UiProjectionError::Invalid("invalid echo area clear rule")),
    };
    Ok(crate::UiEchoPolicy {
        view,
        timeout: std::time::Duration::from_millis(timeout),
        clear_on_input,
    })
}

fn decode_frame_regions(
    value: RootedSchemeObject<'_>,
) -> Result<crate::UiFrameRegions, UiProjectionError> {
    use crate::UiFrameRole;
    let bindings = vector(value, UiFrameRole::ALL.len(), UiFrameRole::ALL.len())?;
    let mut names = IdentifierBuffer::default();
    let mut ids: [Option<UiComponentId>; 7] = std::array::from_fn(|_| None);
    for index in 0..bindings.len() {
        let binding = vector(bindings.element(index)?, 2, 2)?;
        let name = identifier(binding.element(0)?, &mut names)?;
        let role = UiFrameRole::from_name(name.as_str())
            .ok_or(UiProjectionError::Invalid("unknown frame role"))?;
        let id = identifier(binding.element(1)?, &mut names)?;
        if ids[role as usize].replace(id).is_some() {
            return Err(UiProjectionError::Invalid("duplicate frame role"));
        }
    }
    let ids = ids
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or(UiProjectionError::Invalid("missing frame role"))?;
    crate::UiFrameRegions::new(
        ids.try_into()
            .map_err(|_| UiProjectionError::Invalid("missing frame role"))?,
    )
}

fn decode_frame(value: RootedSchemeObject<'_>) -> Result<UiFrameSpec, UiProjectionError> {
    let fields = vector(value, 5, 5)?;
    let rows = |index, upper| -> Result<u16, UiProjectionError> {
        match fields.element(index)?.as_ref().scalar()? {
            SchemeScalar::Fixnum(value) if (0..=upper).contains(&value) => Ok(value as u16),
            _ => Err(UiProjectionError::Invalid("invalid UI frame rows")),
        }
    };
    let boolean = |index| -> Result<bool, UiProjectionError> {
        match fields.element(index)?.as_ref().scalar()? {
            SchemeScalar::Boolean(value) => Ok(value),
            _ => Err(UiProjectionError::Invalid("invalid UI frame flag")),
        }
    };
    let prompt = string(fields.element(4)?)?;
    if prompt.len() > 32 || prompt.chars().any(char::is_control) {
        return Err(UiProjectionError::Invalid("invalid UI frame prompt"));
    }
    Ok(UiFrameSpec {
        min_transcript_rows: rows(0, 4)?,
        fixed_editor_rows: rows(1, 2)?,
        footer_rows: rows(2, 1)?,
        extensions: boolean(3)?,
        prompt,
    })
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
    let mut modeline_slots = [false; MODELINE_SLOTS];
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
            "transcript" if id.as_str() == "transcript" => {
                vector(payload, 0, 0)?;
                UiRegion::Transcript
            }
            "composer" if id.as_str() == "composer" => {
                vector(payload, 0, 0)?;
                UiRegion::Composer
            }
            "commands" if id.as_str() == "commands" => decode_commands(payload)?,
            "status" if id.as_str() == "status" => {
                decode_status(payload, &mut ids, &mut names, &mut modeline_slots)?
            }
            "extension" => decode_extension(id, payload)?,
            _ => return Err(UiProjectionError::Invalid("invalid workbench region")),
        };
        result.push(node);
    }
    // Region order is presentation data. Admit the host capabilities once,
    // regardless of where a Scheme package places the corresponding nodes.
    let role_count = result
        .iter()
        .filter(|region| !matches!(region, UiRegion::Extension { .. }))
        .count();
    if role_count != 4 {
        return Err(UiProjectionError::Invalid("missing workbench capability"));
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
        let row = vector(entries.element(i)?, 5, 5)?;
        let name = UiCommandName::new(&string(row.element(0)?)?)?;
        if !names.insert(name.clone()) {
            return Err(UiProjectionError::Invalid("duplicate command name"));
        }
        let description = string(row.element(1)?)?;
        let mut symbols = IdentifierBuffer::default();
        let arguments = match identifier(row.element(2)?, &mut symbols)?.as_str() {
            "none" => UiCommandArguments::None,
            "required" => UiCommandArguments::Required,
            "optional" => UiCommandArguments::Optional,
            _ => return Err(UiProjectionError::Invalid("invalid command argument rule")),
        };
        let capability = match identifier(row.element(3)?, &mut symbols)?.as_str() {
            "models" => UiCommandCapability::Models,
            "model" => UiCommandCapability::Model,
            "sessions" => UiCommandCapability::Sessions,
            "session" => UiCommandCapability::Session,
            "inspect" => UiCommandCapability::Inspect,
            "history" => UiCommandCapability::History,
            "workspace" => UiCommandCapability::Workspace,
            "context-budget" => UiCommandCapability::ContextBudget,
            "runtime" => UiCommandCapability::Runtime,
            "open-menu" => UiCommandCapability::OpenMenu,
            "select-profile" => UiCommandCapability::SelectProfile,
            "select-buffer" => UiCommandCapability::SelectBuffer,
            "enter-transient" => UiCommandCapability::EnterTransient,
            "kill-process" => UiCommandCapability::KillProcess,
            "envrc" => UiCommandCapability::Envrc,
            "toggle-thinking" => UiCommandCapability::ToggleThinking,
            "clear" => UiCommandCapability::Clear,
            "helm" => UiCommandCapability::Helm,
            "deferred" => UiCommandCapability::Deferred,
            _ => return Err(UiProjectionError::Invalid("unknown command capability")),
        };
        let value = match row.element(4)?.as_ref().kind()? {
            ObjectKind::Boolean
                if matches!(
                    row.element(4)?.as_ref().scalar()?,
                    SchemeScalar::Boolean(false)
                ) =>
            {
                None
            }
            ObjectKind::String => Some(string(row.element(4)?)?),
            _ => return Err(UiProjectionError::Invalid("invalid command fixed value")),
        };
        commands.push(UiCommand {
            name,
            description,
            arguments,
            capability,
            value,
        });
    }
    Ok(UiRegion::Commands(commands))
}

fn valid_dynamic_modeline_template(value: &str) -> bool {
    value.split_once("{value}").is_some_and(|(prefix, suffix)| {
        !prefix
            .chars()
            .chain(suffix.chars())
            .any(|character| matches!(character, '{' | '}'))
    })
}

fn decode_status(
    payload: RootedSchemeObject<'_>,
    ids: &mut HashSet<String>,
    names: &mut IdentifierBuffer,
    modeline_slots: &mut [bool; MODELINE_SLOTS],
) -> Result<UiRegion, UiProjectionError> {
    let children = vector(payload, 0, MAX_STATUS)?;
    let mut items = Vec::with_capacity(children.len());
    for item_index in 0..children.len() {
        let item = vector(children.element(item_index)?, 9, 9)?;
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
            "modeline/state/runtime" => Some(8),
            "modeline/state/workspace" => Some(9),
            "modeline/state/recovery" => Some(10),
            "modeline/state/continuation" => Some(11),
            "modeline/state/history" => Some(12),
            "modeline/state/working" => Some(13),
            "modeline/state/reading" => Some(14),
            _ => None,
        } {
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
        let width = |index| -> Result<u16, UiProjectionError> {
            match item.element(index)?.as_ref().scalar()? {
                SchemeScalar::Fixnum(value) if (0..=256).contains(&value) => Ok(value as u16),
                _ => Err(UiProjectionError::Invalid("invalid modeline width policy")),
            }
        };
        let min_width = width(6)?;
        let compact_below = width(7)?;
        let keep_priority = match item.element(8)?.as_ref().scalar()? {
            SchemeScalar::Fixnum(value) if (0..=3).contains(&value) => value as u8,
            _ => return Err(UiProjectionError::Invalid("invalid modeline priority")),
        };
        if id.as_str().starts_with("modeline/") {
            let dynamic = matches!(
                id.as_str(),
                "modeline/left/model" | "modeline/right/turns" | "modeline/right/session"
            );
            let invalid_presentation = if dynamic {
                !valid_dynamic_modeline_template(&label)
                    || !valid_dynamic_modeline_template(&compact_label)
                    || keep_priority == 0
            } else {
                label.is_empty() || compact_label.is_empty() || keep_priority != 0
            };
            if invalid_presentation {
                return Err(UiProjectionError::Invalid("invalid modeline presentation"));
            }
        } else if compact_label != label
            || tone != UiStatusTone::Muted
            || active_tone != UiStatusTone::Muted
            || min_width != 0
            || compact_below != 0
            || keep_priority != 0
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
            min_width,
            compact_below,
            keep_priority,
        });
    }
    Ok(UiRegion::Status(items))
}

fn decode_extension(
    id: UiComponentId,
    payload: RootedSchemeObject<'_>,
) -> Result<UiRegion, UiProjectionError> {
    let data = vector(payload, 3, 3)?;
    let label = string(data.element(0)?)?;
    let focusable = match data.element(1)?.as_ref().scalar()? {
        SchemeScalar::Boolean(value) => value,
        _ => return Err(UiProjectionError::Invalid("invalid focusable value")),
    };
    let content = crate::decode_element(data.element(2)?)?;
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

pub(crate) fn identifier(
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
