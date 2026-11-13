// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Generic Ratatui input and panel mappings.

use super::{UiRenderInputs, UiSlotRenderer, resolve_choice, terminal_text, tone_color};
use marlin_agent_ui::{UiBackground, UiComponentId, UiContent, UiElement, UiTheme, UiTone};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

/// Measure borders enclosing the selected input before measuring its text.
pub fn input_border(element: &UiElement, facts: &dyn Fn(&UiComponentId) -> bool) -> Option<u16> {
    match element {
        UiElement::Input { bordered, .. } => Some(if *bordered { 2 } else { 0 }),
        UiElement::Choice { .. } => input_border(resolve_choice(element, facts), facts),
        UiElement::Panel { child, .. } => {
            input_border(child, facts).map(|size| size.saturating_add(2))
        }
        UiElement::Sized { child, .. } | UiElement::Region { child, .. } => {
            input_border(child, facts)
        }
        UiElement::Rows(children) | UiElement::Columns(children) => {
            children.iter().find_map(|child| input_border(child, facts))
        }
        UiElement::Text(_) | UiElement::Template { .. } | UiElement::Slot(_) => None,
    }
}

/// One host-provided row and the named values used by its Scheme template.
pub struct UiListRow<'a> {
    pub bindings: Vec<(&'a str, std::borrow::Cow<'a, str>)>,
}

/// Paint a bounded window of a declarative list, keeping its selection visible.
pub fn render_list<'a>(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    template: &UiElement,
    theme: &UiTheme,
    count: usize,
    selected: Option<usize>,
    mut row: impl FnMut(usize) -> Option<UiListRow<'a>>,
) {
    let area = area.intersection(frame.area());
    let visible = usize::from(area.height);
    let selected = selected.filter(|&index| index < count);
    let start = selected.map_or(0, |index| index.saturating_add(1).saturating_sub(visible));
    for index in start..count.min(start.saturating_add(visible)) {
        let Some(row) = row(index) else { continue };
        let line = Rect::new(area.x, area.y + (index - start) as u16, area.width, 1);
        let bindings: Vec<_> = row
            .bindings
            .iter()
            .map(|(key, value)| (*key, value.as_ref()))
            .collect();
        let inputs = UiRenderInputs {
            theme,
            offset: &|_| 0,
            facts: &|id| id.as_str() == "selected" && selected == Some(index),
            extent: &|_| super::Extent::Cells(0),
            bindings: &bindings,
        };
        super::render_with_facts(frame, line, template, &mut Vec::new(), &inputs);
    }
}

/// Bind a single host content source to a declarative slot.
pub struct UiContentSlot<'a> {
    pub title: &'a str,
    pub content: &'a [UiContent],
    pub theme: &'a UiTheme,
    pub scroll: usize,
}

impl UiSlotRenderer for UiContentSlot<'_> {
    fn title(&self, _: &UiComponentId) -> Option<&str> {
        Some(self.title)
    }

    fn render(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        _: &UiComponentId,
        field_tone: Option<UiTone>,
        skip: usize,
    ) {
        render_content(
            frame,
            area,
            self.content,
            self.theme,
            field_tone.unwrap_or(UiTone::Normal),
            self.scroll.saturating_add(skip),
        );
    }
}

/// Paint bounded host content inside a Scheme-selected container.
pub fn render_content(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    content: &[UiContent],
    theme: &UiTheme,
    field_tone: UiTone,
    scroll: usize,
) {
    let clean = |value: &str, limit| {
        value
            .chars()
            .take(limit)
            .map(|ch| if ch.is_control() { '�' } else { ch })
            .collect::<String>()
    };
    let lines: Vec<Line<'static>> = content
        .iter()
        .map(|item| match item {
            UiContent::Text(text) => Line::from(clean(text, 256)),
            UiContent::Field { label, value } => Line::from(vec![
                Span::styled(
                    format!("{}: ", clean(label, 256)),
                    Style::default().fg(tone_color(field_tone, theme)),
                ),
                Span::raw(clean(value, 256)),
            ]),
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
pub fn render_input(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    element: &UiElement,
    inputs: &UiRenderInputs<'_>,
    show_cursor: bool,
    value: impl FnOnce(u16, u16) -> Option<(Vec<String>, (u16, u16))>,
) {
    let element = resolve_choice(element, inputs.facts);
    let UiElement::Input {
        bordered,
        background,
        prefix,
        title,
        active,
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
    let background = match background {
        UiBackground::None => None,
        UiBackground::Canvas => Some(Color::Rgb(
            inputs.theme.canvas.red,
            inputs.theme.canvas.green,
            inputs.theme.canvas.blue,
        )),
        UiBackground::Bar => Some(Color::Rgb(
            inputs.theme.bar.red,
            inputs.theme.bar.green,
            inputs.theme.bar.blue,
        )),
        UiBackground::Tone(tone) => Some(tone_color(*tone, inputs.theme)),
    };
    let style = background.map_or(Style::default(), |color| {
        Style::default()
            .bg(color)
            .fg(tone_color(UiTone::Normal, inputs.theme))
    });
    let mut content = area;
    if *bordered {
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
    if let Some(prefix) = prefix {
        let prefix = terminal_text(binding(prefix));
        let width = UnicodeWidthStr::width(prefix.as_ref()).min(usize::from(content.width)) as u16;
        frame.render_widget(
            Paragraph::new(prefix.into_owned())
                .style(style.fg(tone_color(UiTone::Accent, inputs.theme))),
            content,
        );
        content.x = content.x.saturating_add(width);
        content.width = content.width.saturating_sub(width);
    }
    if content.is_empty() {
        return;
    }
    if let Some((rows, cursor)) = value(content.width, content.height) {
        frame.render_widget(Paragraph::new(rows.join("\n")).style(style), content);
        if show_cursor && cursor.0 < content.width && cursor.1 < content.height {
            frame.set_cursor_position((content.x + cursor.0, content.y + cursor.1));
        }
    }
}
