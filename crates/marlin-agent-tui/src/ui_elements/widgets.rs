// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Generic Ratatui input and panel mappings.

use super::{
    HitRegion, UiRenderInputs, UiSlotRenderer, fits_width, resolve_choice, terminal_text,
    tone_color,
};
use marlin_agent_ui::{UiBarSide, UiComponentId, UiContent, UiElement, UiTheme, UiTone};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::collections::VecDeque;
use std::ops::Range;
use unicode_width::UnicodeWidthStr;

/// Render a bounded flow without assigning meaning to individual segments.
pub(super) fn render_bar(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    element: &UiElement,
    hits: &mut Vec<HitRegion>,
    inputs: &UiRenderInputs<'_>,
) {
    let UiElement::Bar {
        background,
        right_divisor,
        segments,
    } = element
    else {
        return;
    };
    let area = area.intersection(frame.area());
    if area.is_empty() {
        return;
    }
    let right_budget = area.width / (*right_divisor).max(1);
    let facts =
        |id: &UiComponentId| fits_width(id, area.width).unwrap_or_else(|| (inputs.facts)(id));
    let extent = |id: &UiComponentId| {
        if id.as_str() == "bar-right-budget" {
            super::Extent::Cells(right_budget)
        } else {
            (inputs.extent)(id)
        }
    };
    let bound = UiRenderInputs {
        facts: &facts,
        extent: &extent,
        ..*inputs
    };
    let mut measured = Vec::with_capacity(segments.len());
    let mut essential = 0usize;
    let mut essential_right = 0usize;
    for segment in segments {
        let active =
            area.width >= segment.min_width && segment.condition.as_ref().is_none_or(&facts);
        let child = resolve_choice(&segment.child, &facts);
        let width = if active {
            match child {
                UiElement::Template { spans, .. } => spans
                    .iter()
                    .map(|span| {
                        UnicodeWidthStr::width(
                            super::core::template_text(&span.value, &bound, area.width).as_ref(),
                        )
                    })
                    .fold(0usize, usize::saturating_add),
                _ => 0,
            }
        } else {
            0
        };
        let selected = active && segment.priority == 0 && width > 0;
        if selected {
            essential = essential.saturating_add(width);
            if segment.side == UiBarSide::Right {
                essential_right = essential_right.saturating_add(width);
            }
        }
        measured.push((child, width, selected));
    }
    let mut remaining = usize::from(area.width).saturating_sub(essential);
    let mut right_width = essential_right.min(usize::from(area.width));
    for priority in (1..=3).rev() {
        for (segment, (_, width, selected)) in segments.iter().zip(measured.iter_mut()) {
            if segment.priority == priority
                && *width > 0
                && *width <= remaining
                && (segment.side != UiBarSide::Right
                    || right_width.saturating_add(*width) <= usize::from(right_budget))
            {
                *selected = true;
                remaining -= *width;
                if segment.side == UiBarSide::Right {
                    right_width += *width;
                }
            }
        }
    }
    let style = super::core::background_color(*background, inputs.theme)
        .map_or(Style::default(), |color| Style::default().bg(color));
    frame.render_widget(Paragraph::new("").style(style), area);
    let mut left = 0usize;
    let mut right = usize::from(area.width).saturating_sub(right_width);
    for (segment, (child, width, selected)) in segments.iter().zip(measured) {
        if !selected {
            continue;
        }
        let position = if segment.side == UiBarSide::Left {
            &mut left
        } else {
            &mut right
        };
        let cap = if segment.side == UiBarSide::Left {
            usize::from(area.width).saturating_sub(right_width)
        } else {
            usize::from(area.width)
        };
        let visible = width.min(cap.saturating_sub(*position));
        if visible > 0 {
            let rect = Rect::new(area.x + *position as u16, area.y, visible as u16, 1);
            super::render_template(frame, rect, child, &bound);
            if let Some(command) = &segment.command {
                hits.push(HitRegion::command(command, rect));
            }
            *position += visible;
        }
    }
}

#[derive(Default)]
pub(crate) struct InputInsets {
    pub(crate) border: u16,
    trailing: u16,
}

impl InputInsets {
    pub(crate) fn text_width(&self, width: u16, bordered: bool) -> u16 {
        let inside = width.saturating_sub(if bordered { self.border } else { 0 });
        inside - adornment_width(self.trailing, inside)
    }
}

fn adornment_width(cells: u16, width: u16) -> u16 {
    cells.min(width / 3)
}

/// Find the selected input and measure the space reserved around its value.
pub(crate) fn input_insets(
    element: &UiElement,
    facts: &dyn Fn(&UiComponentId) -> bool,
) -> Option<InputInsets> {
    match element {
        UiElement::Input {
            bordered, trailing, ..
        } => Some(InputInsets {
            border: if *bordered { 2 } else { 0 },
            trailing: trailing.as_ref().map_or(0, |adornment| adornment.cells),
        }),
        UiElement::Choice { .. } => input_insets(resolve_choice(element, facts), facts),
        UiElement::Panel {
            border_tone, child, ..
        } => input_insets(child, facts).map(|mut insets| {
            insets.border = insets
                .border
                .saturating_add(if border_tone.is_some() { 2 } else { 0 });
            insets
        }),
        UiElement::Sized { child, .. }
        | UiElement::Region { child, .. }
        | UiElement::List { child, .. } => input_insets(child, facts),
        UiElement::Rows(children) | UiElement::Columns(children) => {
            children.iter().find_map(|child| input_insets(child, facts))
        }
        UiElement::Text(_)
        | UiElement::Template { .. }
        | UiElement::Prompt { .. }
        | UiElement::Slot(_)
        | UiElement::Bar { .. } => None,
    }
}

fn inline_width(element: &UiElement, inputs: &UiRenderInputs<'_>, width: u16) -> u16 {
    let UiElement::Template { spans, .. } = element else {
        return 0;
    };
    spans
        .iter()
        .map(|span| {
            UnicodeWidthStr::width(super::core::template_text(&span.value, inputs, width).as_ref())
        })
        .sum::<usize>()
        .min(usize::from(width)) as u16
}

/// One host-provided row and the named values used by its Scheme template.
pub struct UiListRow<'a> {
    /// Stable identity for hit testing; rows without an identity are display-only.
    pub key: Option<UiComponentId>,
    pub bindings: Vec<(&'a str, std::borrow::Cow<'a, str>)>,
}

/// Host data needed to evaluate a declarative list window.
#[derive(Clone, Copy)]
pub struct UiListData<'a> {
    pub count: usize,
    pub selected: Option<usize>,
    pub extent: &'a dyn Fn(&UiComponentId) -> super::Extent,
    pub facts: &'a dyn Fn(&UiComponentId) -> bool,
    pub bindings: &'a [(&'a str, &'a str)],
}

/// Frame-independent offsets for Scheme-declared lists, bounded per session.
#[derive(Default)]
pub struct UiListState {
    offsets: VecDeque<(UiComponentId, u16)>,
    hits: Vec<UiListHit>,
}

/// One painted row, identified independently of its current list position.
pub struct UiListHit {
    pub list: UiComponentId,
    pub selects_command: bool,
    pub key: UiComponentId,
    pub area: Rect,
}

impl UiListState {
    /// Discard geometry from the previous painted frame.
    pub fn begin_frame(&mut self) {
        self.hits.clear();
    }

    /// Resolve only a row actually painted in the current frame.
    pub fn hit(&self, x: u16, y: u16) -> Option<&UiListHit> {
        self.hits.iter().rev().find(|hit| {
            x >= hit.area.x && x < hit.area.right() && y >= hit.area.y && y < hit.area.bottom()
        })
    }

    fn get(&self, id: &UiComponentId) -> u16 {
        self.offsets
            .iter()
            .find_map(|(key, offset)| (key == id).then_some(*offset))
            .unwrap_or(0)
    }

    fn set(&mut self, id: &UiComponentId, offset: u16) {
        if let Some(index) = self.offsets.iter().position(|(key, _)| key == id) {
            self.offsets.remove(index);
        } else if self.offsets.len() == 64 {
            self.offsets.pop_front();
        }
        self.offsets.push_back((id.clone(), offset));
    }
}

/// Paint a bounded window of a declarative list, keeping its selection visible.
pub fn render_list<'a>(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    template: &UiElement,
    theme: &UiTheme,
    data: UiListData<'_>,
    state: &mut UiListState,
    mut row: impl FnMut(usize) -> Option<UiListRow<'a>>,
) {
    let UiElement::List {
        id,
        source,
        offset,
        child,
    } = template
    else {
        return;
    };
    state.hits.retain(|hit| &hit.list != id);
    let area = area.intersection(frame.area());
    if area.is_empty() {
        return;
    }
    let visible = usize::from(area.height);
    let count = data.count;
    let selected = data.selected.filter(|&index| index < count);
    let cells = |value: usize| super::Extent::Cells(u16::try_from(value).unwrap_or(u16::MAX));
    let saved_offset = state.get(id);
    let metrics = |key: &UiComponentId| match key.as_str() {
        "list-total" => cells(count),
        "list-visible" => cells(visible),
        "list-index" => cells(selected.map_or(0, |idx| idx)),
        "list-offset" => super::Extent::Cells(saved_offset),
        _ => (data.extent)(key),
    };
    let facts = |key: &UiComponentId| match key.as_str() {
        "list-selected" => selected.is_some(),
        _ => (data.facts)(key),
    };
    let start = match super::resolve_size(offset, &metrics, &facts) {
        super::Extent::Cells(value) => usize::from(value).min(count.saturating_sub(visible)),
        super::Extent::Flex(_) => 0,
    };
    state.set(id, start as u16);
    for index in start..count.min(start.saturating_add(visible)) {
        let Some(row) = row(index) else { continue };
        let line = Rect::new(area.x, area.y + (index - start) as u16, area.width, 1);
        if let Some(key) = row.key {
            state.hits.push(UiListHit {
                list: id.clone(),
                selects_command: *source == marlin_agent_ui::UiListSource::Candidates,
                key,
                area: line,
            });
        }
        let mut bindings = Vec::with_capacity(row.bindings.len() + data.bindings.len());
        bindings.extend(
            row.bindings
                .iter()
                .map(|(key, value)| (*key, value.as_ref())),
        );
        // Row bindings shadow frame bindings, while unrelated frame values
        // remain available to the Scheme-declared row template.
        bindings.extend_from_slice(data.bindings);
        let inputs = UiRenderInputs {
            theme,
            offset: &|_| 0,
            facts: &|id| match id.as_str() {
                "selected" => selected == Some(index),
                _ => facts(id),
            },
            extent: &metrics,
            bindings: &bindings,
        };
        super::render_with_facts(frame, line, child, &mut Vec::new(), &inputs);
    }
}

pub(super) fn render_bound_rows(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    element: &UiElement,
    inputs: &UiRenderInputs<'_>,
    state: Option<&std::cell::RefCell<UiListState>>,
    rows: &[marlin_agent_ui::UiListValue],
) {
    let local_state = std::cell::RefCell::new(UiListState::default());
    let mut state = state.unwrap_or(&local_state).borrow_mut();
    render_list(
        frame,
        area,
        element,
        inputs.theme,
        UiListData {
            count: rows.len(),
            selected: None,
            extent: inputs.extent,
            facts: inputs.facts,
            bindings: inputs.bindings,
        },
        &mut state,
        |index| {
            let row = rows.get(index)?;
            Some(UiListRow {
                key: row.id.clone(),
                bindings: row
                    .fields
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str().into()))
                    .collect(),
            })
        },
    );
}

/// Bind a single host content source to a declarative slot.
pub struct UiContentSlot<'a> {
    pub content: &'a [UiContent],
    pub scroll: usize,
}

impl UiSlotRenderer for UiContentSlot<'_> {
    fn render(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        _: &marlin_agent_ui::UiSlotSource,
        field_template: Option<&UiElement>,
        inputs: &UiRenderInputs<'_>,
        skip: usize,
    ) {
        if let Some(field_template) = field_template {
            render_content(
                frame,
                area,
                self.content,
                field_template,
                inputs,
                self.scroll.saturating_add(skip),
            );
        }
    }
}

/// Paint bounded host content inside a Scheme-selected container.
pub fn render_content(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    content: &[UiContent],
    field_template: &UiElement,
    inputs: &UiRenderInputs<'_>,
    scroll: usize,
) {
    let field_template = resolve_choice(field_template, inputs.facts);
    let lines: Vec<Line<'static>> = content
        .iter()
        .map(|item| match item {
            UiContent::Text(text) => Line::from(super::core::style_structured_org_line(
                text,
                Style::default().fg(tone_color(marlin_agent_ui::UiTone::Normal, inputs.theme)),
                inputs.theme,
            )),
            UiContent::Field { label, value } => {
                let UiElement::Template { spans, background } = field_template else {
                    return Line::default();
                };
                let mut bindings = Vec::with_capacity(inputs.bindings.len() + 2);
                bindings.extend([("label", label.as_str()), ("value", value.as_str())]);
                bindings.extend_from_slice(inputs.bindings);
                let row = UiRenderInputs {
                    bindings: &bindings,
                    ..*inputs
                };
                let line = Line::from(
                    spans
                        .iter()
                        .map(|span| {
                            let mut style =
                                Style::default().fg(tone_color(span.tone, inputs.theme));
                            if span.bold {
                                style = style.add_modifier(ratatui::style::Modifier::BOLD);
                            }
                            Span::styled(
                                super::core::template_text(&span.value, &row, area.width)
                                    .into_owned(),
                                style,
                            )
                        })
                        .collect::<Vec<_>>(),
                );
                match super::core::background_color(*background, inputs.theme) {
                    Some(color) => line.style(Style::default().bg(color)),
                    None => line,
                }
            }
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((scroll.min(u16::MAX as usize) as u16, 0)),
        area,
    );
}

/// Render an admitted input component using a host-provided editable value.
/// Visible input rows, cursor, and selected cell spans in row order.
#[derive(Debug, PartialEq, Eq)]
pub struct UiInputValue {
    pub rows: Vec<String>,
    pub cursor: (u16, u16),
    pub selection: Vec<(u16, Range<u16>)>,
    pub marks: Vec<UiInputMark>,
}

/// A cell annotation that occupies one complete visual grapheme.
#[derive(Debug, PartialEq, Eq)]
pub struct UiInputMark {
    pub row: u16,
    pub column: u16,
    pub cells: u16,
    pub label: char,
    pub tone: UiTone,
}

impl From<(Vec<String>, (u16, u16))> for UiInputValue {
    fn from((rows, cursor): (Vec<String>, (u16, u16))) -> Self {
        Self {
            rows,
            cursor,
            selection: Vec::new(),
            marks: Vec::new(),
        }
    }
}

pub fn render_input<Value: Into<UiInputValue>>(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    element: &UiElement,
    inputs: &UiRenderInputs<'_>,
    show_cursor: bool,
    value: impl FnOnce(u16, u16) -> Option<Value>,
) {
    let element = resolve_choice(element, inputs.facts);
    let UiElement::Input {
        bordered,
        background,
        prefix,
        title,
        active,
        trailing,
    } = element
    else {
        return;
    };
    let area = area.intersection(frame.area());
    if area.is_empty() {
        return;
    }
    let binding = |name: &UiComponentId| {
        inputs
            .bindings
            .iter()
            .find_map(|(key, value)| (*key == name.as_str()).then_some(*value))
            .unwrap_or("")
    };
    let background = super::core::background_color(*background, inputs.theme);
    let style = background.map_or(Style::default(), |color| {
        Style::default()
            .bg(color)
            .fg(tone_color(UiTone::Normal, inputs.theme))
    });
    let mut content = area;
    // A border has no interior in a terminal allocation below three cells.
    if *bordered && area.width > 2 && area.height > 2 {
        let title = title.as_ref().map(&binding).unwrap_or("");
        let block = Block::default()
            .borders(Borders::ALL)
            .title(terminal_text(title).into_owned())
            .border_style(if (inputs.facts)(active) {
                Style::default().fg(tone_color(UiTone::Info, inputs.theme))
            } else {
                Style::default()
            });
        content = block.inner(area);
        frame.render_widget(block, area);
    } else {
        frame.render_widget(Paragraph::new("").style(style), area);
    }
    let trailing_width = trailing.as_ref().map_or(0, |adornment| {
        adornment_width(adornment.cells, content.width)
    });
    content.width -= trailing_width;
    let mut trailing_row = content.bottom().saturating_sub(1);
    if let Some(prefix) = prefix {
        let prefix_facts = |id: &UiComponentId| {
            fits_width(id, content.width).unwrap_or_else(|| (inputs.facts)(id))
        };
        let prefix = resolve_choice(prefix, &prefix_facts);
        let prefix_inputs = UiRenderInputs {
            facts: &prefix_facts,
            ..*inputs
        };
        let width = inline_width(prefix, &prefix_inputs, content.width);
        super::render_template(
            frame,
            Rect::new(content.x, content.y, width, 1),
            prefix,
            &prefix_inputs,
        );
        content.x = content.x.saturating_add(width);
        content.width = content.width.saturating_sub(width);
    }
    if content.is_empty() {
        return;
    }
    if let Some(value) = value(content.width, content.height) {
        let UiInputValue {
            rows,
            cursor,
            selection,
            marks,
        } = value.into();
        frame.render_widget(Paragraph::new(rows.join("\n")).style(style), content);
        for (row, columns) in selection {
            let end = columns.end.min(content.width);
            if row < content.height && columns.start < end {
                frame.buffer_mut().set_style(
                    Rect::new(
                        content.x + columns.start,
                        content.y + row,
                        end - columns.start,
                        1,
                    ),
                    Style::default().add_modifier(Modifier::REVERSED),
                );
            }
        }
        for mark in marks {
            if mark.row >= content.height
                || mark.column >= content.width
                || mark.cells == 0
                || !mark.label.is_ascii_graphic()
            {
                continue;
            }
            let x = content.x + mark.column;
            let y = content.y + mark.row;
            let style = Style::default()
                .fg(tone_color(mark.tone, inputs.theme))
                .add_modifier(Modifier::BOLD);
            let buffer = frame.buffer_mut();
            buffer[(x, y)].set_char(mark.label).set_style(style);
            for column in 1..mark.cells.min(content.width - mark.column) {
                buffer[(x + column, y)].set_char(' ').set_style(style);
            }
        }
        if show_cursor && cursor.0 < content.width && cursor.1 < content.height {
            trailing_row = content.y + cursor.1;
            frame.set_cursor_position((content.x + cursor.0, trailing_row));
        }
    }
    if let Some(trailing) = trailing {
        let view = resolve_choice(&trailing.view, inputs.facts);
        let width = inline_width(view, inputs, trailing_width);
        super::render_template(
            frame,
            Rect::new(
                content.right() + (trailing_width - width),
                trailing_row,
                width,
                1,
            ),
            view,
            inputs,
        );
    }
}
