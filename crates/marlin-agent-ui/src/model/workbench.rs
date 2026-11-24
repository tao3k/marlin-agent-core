// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Workbench snapshots and host admission for Scheme transitions.

use gerbil_scheme::RootedSchemeObject;

use super::{
    MODELINE_SLOTS, UiCommand, UiComponentId, UiEvent, UiFrame, UiIntent, UiKeyBinding,
    UiKeyCommand, UiKeyMode, UiKeyScope, UiLayoutPolicy, UiProjectionError, UiRegion,
    UiScrollDirection, UiStatusItem, UiTheme, UiTransient,
};

/// Host display facts and already-rooted Scheme extension contributions.
/// The selected publication and extension provenance remain the VM/module
/// owner's responsibility.
pub struct UiOpenFacts<'a, 'runtime> {
    pub model_label: &'a str,
    pub session_label: &'a str,
    pub status: &'a [UiStatusItem],
    pub extensions: &'a [RootedSchemeObject<'runtime>],
    /// Scheme thunks returning fresh method bundles for this session.
    pub method_factories: &'a [RootedSchemeObject<'runtime>],
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
    pub fn key_command(&self, sequence: &[&str], mode: Option<UiKeyMode>) -> Option<UiKeyCommand> {
        self.keymap
            .iter()
            .find(|binding| {
                (binding.scope == UiKeyScope::Global
                    || mode.is_some_and(|mode| binding.scope == UiKeyScope::Mode(mode)))
                    && binding.sequence.len() == sequence.len()
                    && binding
                        .sequence
                        .iter()
                        .zip(sequence)
                        .all(|(left, right)| left.as_str() == *right)
            })
            .map(|binding| binding.command.clone())
    }

    pub fn key_prefix(&self, sequence: &[&str], mode: Option<UiKeyMode>) -> bool {
        self.keymap.iter().any(|binding| {
            (binding.scope == UiKeyScope::Global
                || mode.is_some_and(|mode| binding.scope == UiKeyScope::Mode(mode)))
                && binding.sequence.len() > sequence.len()
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
                UiEvent::NextFocus
                    | UiEvent::PreviousFocus
                    | UiEvent::Dismiss
                    | UiEvent::SelectListRow { .. },
                UiIntent::Focus(_)
            ) | (UiEvent::ScrollUp, UiIntent::Scroll(UiScrollDirection::Up))
                | (
                    UiEvent::ScrollDown,
                    UiIntent::Scroll(UiScrollDirection::Down)
                )
        ) || matches!((&event, intent), (UiEvent::SelectFocus(requested), UiIntent::Focus(id))
            if requested == id)
            || (profile_change && matches!(intent, UiIntent::Focus(id) if id == &self.focus))
            || (matches!(&event, UiEvent::RecordCommand(name) if self.commands().iter().any(|command| &command.name == name))
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
                    || self.layout.regions != next.layout.regions
                    || self.layout.command_frame != next.layout.command_frame
                    || self.layout.max_chatbox_rows != next.layout.max_chatbox_rows
                    || self.layout.max_command_rows != next.layout.max_command_rows
                    || !self
                        .layout
                        .frame_tree
                        .same_profile_structure(&next.layout.frame_tree)
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
