// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Word and logical-line endpoints shared by cursor movement and input markup.

use std::num::NonZeroU8;

use marlin_agent_ui::{
    UiCharacterSelection, UiEditorAction, UiKeyBinding, UiKeyCommand, UiKeyMode, UiKeyScope, UiTone,
};
use unicode_segmentation::UnicodeSegmentation;

use super::{Composer, word_class};
use crate::ui_elements::UiInputMark;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ExpansionUnit {
    Word,
    Line,
    Character { target: char, inclusive: bool },
}

#[derive(Clone, Copy)]
pub(super) struct OffsetMark {
    offset: usize,
    label: char,
    tone: UiTone,
}

impl Composer {
    pub(crate) fn select_character(&mut self, target: char, selection: UiCharacterSelection) {
        let unit = ExpansionUnit::Character {
            target,
            inclusive: selection.inclusive,
        };
        let Some(destination) = next_endpoint(&self.text, self.cursor, unit, selection.backward)
        else {
            return;
        };
        self.change_selection(|editor| {
            editor.anchor = Some(editor.cursor);
            editor.cursor = destination;
            editor.expansion_unit = Some(unit);
            editor.vertical_column = None;
            editor.undo_insert_group = false;
        });
    }

    fn expansion_points(&self) -> Option<impl Iterator<Item = usize> + '_> {
        let anchor = self.anchor?;
        let unit = self.expansion_unit?;
        if anchor == self.cursor && unit == ExpansionUnit::Word {
            return None;
        }
        Some(
            std::iter::successors(Some(self.cursor), move |&offset| {
                next_endpoint(&self.text, offset, unit, anchor > self.cursor)
            })
            .skip(1),
        )
    }

    /// Whether expansion applies to this selection, even at a draft edge.
    pub(crate) fn expand_selection(&mut self, steps: NonZeroU8) -> bool {
        let Some(destination) = self
            .expansion_points()
            .map(|mut points| points.nth(usize::from(steps.get()) - 1))
        else {
            return false;
        };
        if let Some(destination) = destination {
            self.change_selection(|editor| {
                editor.cursor = destination;
                editor.vertical_column = None;
                editor.undo_insert_group = false;
            });
        }
        true
    }

    pub(super) fn expansion_marks(&self, bindings: &[UiKeyBinding]) -> Vec<OffsetMark> {
        if bindings.is_empty() {
            return Vec::new();
        }
        let Some(points) = self.expansion_points() else {
            return Vec::new();
        };
        let mut labels = [None; u8::MAX as usize];
        let mut last_step = 0;
        for binding in bindings {
            if !matches!(
                binding.scope,
                UiKeyScope::Mode(UiKeyMode::Normal) | UiKeyScope::Global
            ) {
                continue;
            }
            if let (UiKeyCommand::Edit(UiEditorAction::ExpandSelection { steps, tone, .. }), [key]) =
                (&binding.command, binding.sequence.as_slice())
                && let [label] = key.as_str().as_bytes()
                && label.is_ascii_graphic()
            {
                let step = usize::from(steps.get());
                labels[step - 1].get_or_insert((char::from(*label), *tone));
                last_step = last_step.max(step);
            }
        }
        let mut marks: Vec<_> = labels[..last_step]
            .iter()
            .copied()
            .zip(points)
            .filter_map(|(label, offset)| {
                let (label, tone) = label?;
                Some(OffsetMark {
                    offset,
                    label,
                    tone,
                })
            })
            .collect();
        marks.sort_unstable_by_key(|mark| mark.offset);
        marks
    }
}

fn next_endpoint(text: &str, offset: usize, unit: ExpansionUnit, backward: bool) -> Option<usize> {
    match (unit, backward) {
        (ExpansionUnit::Word, false) => text[offset..]
            .grapheme_indices(true)
            .skip_while(|(_, grapheme)| word_class(grapheme) != 1)
            .take_while(|(_, grapheme)| word_class(grapheme) == 1)
            .last()
            .map(|(start, grapheme)| offset + start + grapheme.len()),
        (ExpansionUnit::Word, true) => text[..offset]
            .grapheme_indices(true)
            .rev()
            .skip_while(|(_, grapheme)| word_class(grapheme) != 1)
            .take_while(|(_, grapheme)| word_class(grapheme) == 1)
            .last()
            .map(|(start, _)| start),
        (ExpansionUnit::Line, false) => {
            let start = offset + text[offset..].find('\n')? + 1;
            Some(
                text[start..]
                    .find('\n')
                    .map_or(text.len(), |end| start + end),
            )
        }
        (ExpansionUnit::Line, true) => {
            let separator = text[..offset].rfind('\n')?;
            Some(text[..separator].rfind('\n').map_or(0, |index| index + 1))
        }
        (ExpansionUnit::Character { target, inclusive }, false) => text[offset..]
            .grapheme_indices(true)
            .skip(usize::from(!inclusive))
            .find(|(_, grapheme)| grapheme.contains(target))
            .map(|(index, grapheme)| offset + index + if inclusive { grapheme.len() } else { 0 }),
        (ExpansionUnit::Character { target, inclusive }, true) => text[..offset]
            .grapheme_indices(true)
            .rev()
            .skip(usize::from(!inclusive))
            .find(|(_, grapheme)| grapheme.contains(target))
            .map(|(index, grapheme)| index + if inclusive { 0 } else { grapheme.len() }),
    }
}

/// Map sorted byte marks to clipped terminal cells in the existing layout pass.
pub(super) struct MarkLayout<'a> {
    remaining: &'a [OffsetMark],
    pub visible: Vec<UiInputMark>,
}

impl<'a> MarkLayout<'a> {
    pub(super) fn new(remaining: &'a [OffsetMark]) -> Self {
        Self {
            remaining,
            visible: Vec::new(),
        }
    }

    pub(super) fn span(
        &mut self,
        bytes: std::ops::Range<usize>,
        row: usize,
        column: usize,
        cells: usize,
    ) {
        // ASCII runs have one cell per byte. A Unicode grapheme is supplied
        // as a single byte offset with its entire terminal width.
        while let Some((mark, rest)) = self.remaining.split_first() {
            if mark.offset >= bytes.end {
                break;
            }
            self.remaining = rest;
            if bytes.contains(&mark.offset) && cells > 0 {
                self.visible.push(UiInputMark {
                    column: (column + (mark.offset - bytes.start)) as u16,
                    row: row as u16,
                    cells: cells as u16,
                    label: mark.label,
                    tone: mark.tone,
                });
            }
        }
    }
}
