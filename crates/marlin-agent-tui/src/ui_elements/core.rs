// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Generic Ratatui mapping for admitted Scheme UI elements.

use super::geometry::{Extent, HitRegion, layout_scrolled_children, panel_areas};

use marlin_agent_ui::{UiBackground, UiComponentId, UiElement, UiTemplateValue, UiTheme, UiTone};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub(crate) fn fits_width(id: &UiComponentId, width: u16) -> Option<bool> {
    fits_dimension(id, "width-at-least-", width)
}

pub(crate) fn fits_area(id: &UiComponentId, area: Rect) -> Option<bool> {
    fits_width(id, area.width).or_else(|| fits_dimension(id, "height-at-least-", area.height))
}

fn fits_dimension(id: &UiComponentId, prefix: &str, cells: u16) -> Option<bool> {
    id.as_str()
        .strip_prefix(prefix)?
        .parse::<u16>()
        .ok()
        .map(|minimum| cells >= minimum)
}

pub(crate) fn tone_color(tone: UiTone, theme: &UiTheme) -> Color {
    let color = match tone {
        UiTone::Normal => theme.ink,
        UiTone::Canvas => theme.canvas,
        UiTone::Accent => theme.accent,
        UiTone::Info => theme.info,
        UiTone::Success => theme.success,
        UiTone::Warning => theme.warning,
        UiTone::Muted => theme.muted,
    };
    Color::Rgb(color.red, color.green, color.blue)
}

pub(super) fn background_color(background: UiBackground, theme: &UiTheme) -> Option<Color> {
    match background {
        UiBackground::None => None,
        UiBackground::Canvas => Some(Color::Rgb(
            theme.canvas.red,
            theme.canvas.green,
            theme.canvas.blue,
        )),
        UiBackground::Bar => Some(Color::Rgb(theme.bar.red, theme.bar.green, theme.bar.blue)),
        UiBackground::Tone(tone) => Some(tone_color(tone, theme)),
    }
}

pub(super) fn terminal_text(value: &str) -> std::borrow::Cow<'_, str> {
    if value.chars().any(char::is_control) {
        std::borrow::Cow::Owned(
            value
                .chars()
                .map(|ch| if ch.is_control() { '�' } else { ch })
                .collect(),
        )
    } else {
        std::borrow::Cow::Borrowed(value)
    }
}

/// Draw a declarative element within a terminal rectangle.
pub fn render_element(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    element: &UiElement,
    theme: &UiTheme,
) {
    render_with_regions(frame, area, element, theme, &mut Vec::new());
}

/// Clear an overlay rectangle onto the theme canvas and render a declarative `UiElement` tree.
pub fn render_overlay_element(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    element: &UiElement,
    theme: &UiTheme,
) {
    let area = area.intersection(frame.area());
    if area.is_empty() {
        return;
    }
    frame.render_widget(Clear, area);
    frame.buffer_mut().set_style(
        area,
        Style::default()
            .bg(Color::Rgb(
                theme.canvas.red,
                theme.canvas.green,
                theme.canvas.blue,
            ))
            .fg(Color::Rgb(theme.ink.red, theme.ink.green, theme.ink.blue)),
    );
    render_element(frame, area, element, theme);
}

fn is_org_table_separator(trimmed: &str) -> bool {
    trimmed.starts_with('|')
        && trimmed.len() >= 3
        && trimmed
            .bytes()
            .all(|b| matches!(b, b'|' | b'-' | b'+' | b':' | b' '))
}

fn push_inline_org_spans(
    text: &str,
    base: Style,
    theme: &UiTheme,
    out: &mut Vec<Span<'static>>,
) {
    let code_style = Style::default()
        .fg(tone_color(UiTone::Info, theme))
        .bg(Color::Rgb(theme.bar.red, theme.bar.green, theme.bar.blue));
    let bold_style = base.add_modifier(Modifier::BOLD);
    let link_style = Style::default()
        .fg(tone_color(UiTone::Info, theme))
        .add_modifier(Modifier::UNDERLINED);

    let bytes = text.as_bytes();
    let mut i = 0usize;
    let mut plain_start = 0usize;

    let flush_plain = |start: usize, end: usize, out: &mut Vec<Span<'static>>| {
        if start < end {
            out.push(Span::styled(text[start..end].to_owned(), base));
        }
    };

    while i < bytes.len() {
        // Org link `[[target][label]]`
        if bytes[i] == b'['
            && bytes.get(i + 1) == Some(&b'[')
            && let Some(rel_close) = text[i + 2..].find("]]")
        {
            let inner = &text[i + 2..i + 2 + rel_close];
            let label = inner.split_once("][").map_or(inner, |(_, lbl)| lbl);
            if !label.is_empty() {
                flush_plain(plain_start, i, out);
                out.push(Span::styled(label.to_owned(), link_style));
                i = i + 2 + rel_close + 2;
                plain_start = i;
                continue;
            }
        }
        // Inline code/verbatim: `code`, ~code~, =verbatim=
        if matches!(bytes[i], b'`' | b'~' | b'=') {
            let delim = bytes[i] as char;
            let boundary_before = i == 0
                || text[..i]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_whitespace() || matches!(c, '(' | '[' | '{' | ':' | '·' | '—' | '（' | '「'));
            if (delim == '`' || boundary_before)
                && let Some(rel_end) = text[i + 1..].find(delim)
                && rel_end > 0
            {
                let span_end = i + 1 + rel_end;
                flush_plain(plain_start, i, out);
                out.push(Span::styled(text[i..=span_end].to_owned(), code_style));
                i = span_end + 1;
                plain_start = i;
                continue;
            }
        }
        // Bold `**text**`
        if bytes[i] == b'*'
            && bytes.get(i + 1) == Some(&b'*')
            && let Some(rel_end) = text[i + 2..].find("**")
            && rel_end > 0
        {
            let span_end = i + 2 + rel_end;
            flush_plain(plain_start, i, out);
            out.push(Span::styled(
                text[i + 2..span_end].to_owned(),
                bold_style,
            ));
            i = span_end + 2;
            plain_start = i;
            continue;
        }
        i += text[i..].chars().next().map_or(1, char::len_utf8);
    }
    flush_plain(plain_start, text.len(), out);
}

/// Fast single-pass, O(line-length) Org-mode (+ Markdown-compatible) structured
/// line styler using ONLY `UiTheme` semantic tones. Designed to run solely on
/// visible viewport rows (~40 lines/frame) for 120Hz zero-lag streaming.
pub(crate) fn style_structured_org_line(
    raw: &str,
    base: Style,
    theme: &UiTheme,
) -> Vec<Span<'static>> {
    let text = terminal_text(raw);
    let line = text.as_ref();
    let trimmed = line.trim_start();
    let indent_len = line.len() - trimmed.len();
    let indent = &line[..indent_len];

    if trimmed.is_empty() {
        return vec![Span::styled(line.to_owned(), base)];
    }

    let muted_style = Style::default().fg(tone_color(UiTone::Muted, theme));
    let bar_bg = Color::Rgb(theme.bar.red, theme.bar.green, theme.bar.blue);

    // 1. Org headings (`* `, `** `, `*** `) or Markdown headings (`# `, `## `, `### `)
    let heading_marker = if trimmed.starts_with('*') {
        let stars = trimmed.bytes().take_while(|&b| b == b'*').count();
        (stars > 0 && trimmed.as_bytes().get(stars) == Some(&b' ')).then_some(stars)
    } else if trimmed.starts_with('#') {
        let hashes = trimmed.bytes().take_while(|&b| b == b'#').count();
        (hashes > 0 && trimmed.as_bytes().get(hashes) == Some(&b' ')).then_some(hashes)
    } else {
        None
    };
    if let Some(level) = heading_marker {
        let tone = match level {
            1 => UiTone::Accent,
            2 => UiTone::Info,
            _ => UiTone::Success,
        };
        let heading_style = Style::default()
            .fg(tone_color(tone, theme))
            .add_modifier(Modifier::BOLD);
        let mut out = Vec::new();
        if !indent.is_empty() {
            out.push(Span::styled(indent.to_owned(), base));
        }
        let prefix = &trimmed[..=level];
        let rest = &trimmed[level + 1..];
        out.push(Span::styled(prefix.to_owned(), heading_style));
        for (kw, kw_tone) in [
            ("TODO ", UiTone::Warning),
            ("NEXT ", UiTone::Warning),
            ("BLOCKED ", UiTone::Warning),
            ("Failed ", UiTone::Warning),
            ("Running ", UiTone::Info),
            ("DONE ", UiTone::Success),
            ("Completed ", UiTone::Success),
        ] {
            if let Some(after) = rest.strip_prefix(kw) {
                out.push(Span::styled(
                    kw.to_owned(),
                    Style::default()
                        .fg(tone_color(kw_tone, theme))
                        .add_modifier(Modifier::BOLD),
                ));
                out.push(Span::styled(after.to_owned(), heading_style));
                return out;
            }
        }
        out.push(Span::styled(rest.to_owned(), heading_style));
        return out;
    }

    // 2. Org blocks & keywords (`#+BEGIN_SRC`, `#+END_SRC`, `#+TITLE:`, etc.) and fenced code (` ``` `)
    if trimmed.starts_with("#+") || trimmed.starts_with("```") {
        let directive_style = Style::default()
            .fg(tone_color(UiTone::Info, theme))
            .bg(bar_bg)
            .add_modifier(Modifier::BOLD);
        return vec![Span::styled(line.to_owned(), directive_style)];
    }

    // 3. Org drawers (`:PROPERTIES:`, `:END:`, `:KEY: value`)
    if trimmed.starts_with(':')
        && let Some(colon_end) = trimmed[1..].find(':')
        && colon_end > 0
        && trimmed[1..=colon_end]
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        let key_end = colon_end + 2;
        let mut out = Vec::new();
        if !indent.is_empty() {
            out.push(Span::styled(indent.to_owned(), base));
        }
        out.push(Span::styled(
            trimmed[..key_end].to_owned(),
            muted_style.add_modifier(Modifier::BOLD),
        ));
        if key_end < trimmed.len() {
            push_inline_org_spans(&trimmed[key_end..], base, theme, &mut out);
        }
        return out;
    }

    // 4. Org / Markdown tables (`| col | col |` and `|---+---|`)
    if trimmed.starts_with('|') {
        if is_org_table_separator(trimmed) {
            return vec![Span::styled(line.to_owned(), muted_style)];
        }
        let mut out = Vec::new();
        if !indent.is_empty() {
            out.push(Span::styled(indent.to_owned(), base));
        }
        for (idx, part) in trimmed.split('|').enumerate() {
            if idx > 0 {
                out.push(Span::styled("│".to_owned(), muted_style));
            }
            if !part.is_empty() {
                push_inline_org_spans(part, base, theme, &mut out);
            }
        }
        return out;
    }

    // 5. Org / Markdown lists & checkboxes (`- [ ] `, `- [X] `, `- `, `+ `)
    if let Some(after) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("+ "))
    {
        let mut out = Vec::new();
        if !indent.is_empty() {
            out.push(Span::styled(indent.to_owned(), base));
        }
        let bullet_style = Style::default()
            .fg(tone_color(UiTone::Accent, theme))
            .add_modifier(Modifier::BOLD);
        out.push(Span::styled(trimmed[..2].to_owned(), bullet_style));
        if let Some(rest) = after.strip_prefix("[X] ").or_else(|| after.strip_prefix("[x] ")) {
            out.push(Span::styled(
                "[X] ".to_owned(),
                Style::default()
                    .fg(tone_color(UiTone::Success, theme))
                    .add_modifier(Modifier::BOLD),
            ));
            push_inline_org_spans(rest, base, theme, &mut out);
        } else if let Some(rest) = after.strip_prefix("[ ] ") {
            out.push(Span::styled(
                "[ ] ".to_owned(),
                Style::default()
                    .fg(tone_color(UiTone::Warning, theme))
                    .add_modifier(Modifier::BOLD),
            ));
            push_inline_org_spans(rest, base, theme, &mut out);
        } else {
            push_inline_org_spans(after, base, theme, &mut out);
        }
        return out;
    }

    // 6. Standard line with inline Org/Markdown emphasis (`code`, ~code~, =verbatim=, **bold**, [[link][label]])
    let mut out = Vec::new();
    push_inline_org_spans(line, base, theme, &mut out);
    out
}

/// Render a Scheme-declared line with typed, named host values.
pub fn render_template(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    element: &UiElement,
    inputs: &UiRenderInputs<'_>,
) {
    let UiElement::Template { background, spans } = element else {
        return;
    };
    let area = area.intersection(frame.area());
    if area.is_empty() {
        return;
    }
    let mut line: Vec<Span<'_>> = Vec::with_capacity(spans.len());
    for span in spans {
        let text = template_text(&span.value, inputs, area.width);
        let mut style = Style::default().fg(tone_color(span.tone, inputs.theme));
        if span.bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        let is_structured_body = matches!(
            &span.value,
            UiTemplateValue::Binding(id) if id.as_str() == "body-text"
        );
        if is_structured_body && !text.is_empty() {
            line.extend(style_structured_org_line(&text, style, inputs.theme));
        } else {
            line.push(Span::styled(text, style));
        }
    }
    let background = background_color(*background, inputs.theme);
    let style = background.map_or(Style::default(), |color| Style::default().bg(color));
    frame.render_widget(Paragraph::new(Line::from(line)).style(style), area);
}

pub(crate) fn inline_text(element: &UiElement, inputs: &UiRenderInputs<'_>, width: u16) -> String {
    let UiElement::Template { spans, .. } = resolve_choice(element, inputs.facts) else {
        return String::new();
    };
    let mut text = String::new();
    for span in spans {
        text.push_str(&template_text(&span.value, inputs, width));
    }
    text
}

pub(super) fn template_text<'a>(
    value: &'a UiTemplateValue,
    inputs: &UiRenderInputs<'a>,
    limit: u16,
) -> std::borrow::Cow<'a, str> {
    let (value, width, tail) = match value {
        UiTemplateValue::Literal(value) => (value.as_str(), None, false),
        UiTemplateValue::Binding(id) => (
            inputs
                .bindings
                .iter()
                .find_map(|(name, value)| (*name == id.as_str()).then_some(*value))
                .unwrap_or(""),
            None,
            false,
        ),
        UiTemplateValue::Column { id, width } => (
            inputs
                .bindings
                .iter()
                .find_map(|(name, value)| (*name == id.as_str()).then_some(*value))
                .unwrap_or(""),
            Some(width),
            false,
        ),
        UiTemplateValue::Tail { id, width } => (
            inputs
                .bindings
                .iter()
                .find_map(|(name, value)| (*name == id.as_str()).then_some(*value))
                .unwrap_or(""),
            Some(width),
            true,
        ),
    };
    let mut text = terminal_text(value);
    if let Some(width) = width {
        let requested = match super::resolve_size(width, inputs.extent, inputs.facts) {
            Extent::Cells(value) => value.min(limit) as usize,
            Extent::Flex(_) => 0,
        };
        let actual = UnicodeWidthStr::width(text.as_ref());
        if tail && actual > requested {
            if requested == 0 {
                return std::borrow::Cow::Borrowed("");
            }
            let mut used = 0usize;
            let mut start = text.len();
            for (index, grapheme) in text.grapheme_indices(true).rev() {
                let cells = UnicodeWidthStr::width(grapheme);
                if used.saturating_add(cells) > requested - 1 {
                    break;
                }
                used += cells;
                start = index;
            }
            let mut clipped =
                String::with_capacity(text.len().saturating_sub(start) + '…'.len_utf8());
            clipped.push('…');
            clipped.push_str(&text[start..]);
            text = std::borrow::Cow::Owned(clipped);
        } else if !tail {
            let padding = requested.saturating_sub(actual);
            if padding > 0 {
                text.to_mut().extend(std::iter::repeat_n(' ', padding));
            }
        }
    }
    text
}

/// Resolve Scheme's ordered conditional presentation against host-supplied facts.
pub fn resolve_choice<'a>(
    mut element: &'a UiElement,
    facts: &dyn Fn(&UiComponentId) -> bool,
) -> &'a UiElement {
    while let UiElement::Choice { cases, fallback } = element {
        element = cases
            .iter()
            .find_map(|(fact, view)| facts(fact).then_some(view))
            .unwrap_or(fallback);
    }
    element
}

/// Paint a one-line template with a trailing editable field and its cursor.
pub fn render_prompt(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    element: &UiElement,
    inputs: &UiRenderInputs<'_>,
    field: &str,
    value: &str,
) -> Option<(u16, u16)> {
    let selected = resolve_choice(element, inputs.facts);
    let UiElement::Template { spans, .. } = selected else {
        return None;
    };
    let (last, prefix) = spans.split_last()?;
    if !matches!(&last.value, UiTemplateValue::Binding(id) if id.as_str() == field) {
        return None;
    }
    let prefix_width: usize = prefix
        .iter()
        .map(|span| UnicodeWidthStr::width(template_text(&span.value, inputs, area.width).as_ref()))
        .sum();
    let sanitized = terminal_text(value);
    let area = area.intersection(frame.area());
    if area.is_empty() {
        return None;
    }
    let available = usize::from(area.width)
        .saturating_sub(prefix_width)
        .saturating_sub(1);
    let mut start = sanitized.len();
    let mut visible_width = 0usize;
    for (index, grapheme) in sanitized.grapheme_indices(true).rev() {
        let width = UnicodeWidthStr::width(grapheme);
        if visible_width.saturating_add(width) > available {
            break;
        }
        start = index;
        visible_width += width;
    }
    let visible = &sanitized[start..];
    let mut bindings = Vec::with_capacity(inputs.bindings.len() + 1);
    bindings.push((field, visible));
    bindings.extend_from_slice(inputs.bindings);
    let bound = UiRenderInputs {
        bindings: &bindings,
        ..*inputs
    };
    render_template(frame, area, selected, &bound);
    let x = (prefix_width + visible_width).min(usize::from(area.width - 1));
    Some((area.x + x as u16, area.y))
}

/// Render and collect the clipped terminal rectangles for interactive regions.
pub fn render_with_regions(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    element: &UiElement,
    theme: &UiTheme,
    regions: &mut Vec<HitRegion>,
) {
    render_with_scroll(frame, area, element, theme, regions, &|_| 0);
}

/// Draw a tree and apply the caller's per-region vertical scroll offsets.
///
/// Regions are appended in paint order; clear the vector before a new frame.
pub fn render_with_scroll(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    element: &UiElement,
    theme: &UiTheme,
    regions: &mut Vec<HitRegion>,
    offset: &dyn Fn(&UiComponentId) -> usize,
) {
    let inputs = UiRenderInputs {
        theme,
        offset,
        facts: &|_| false,
        extent: &|_| Extent::Cells(0),
        bindings: &[],
    };
    render_with_facts(frame, area, element, regions, &inputs);
}

/// Runtime values for generic presentation and scrolling.
pub struct UiRenderInputs<'a> {
    pub theme: &'a UiTheme,
    pub offset: &'a dyn Fn(&UiComponentId) -> usize,
    pub facts: &'a dyn Fn(&UiComponentId) -> bool,
    pub extent: &'a dyn Fn(&UiComponentId) -> Extent,
    pub bindings: &'a [(&'a str, &'a str)],
}

/// Host data bound to a Scheme-declared slot, independent of its placement.
pub trait UiSlotRenderer {
    /// Bind a Scheme-authored subtree and its current viewport offset.
    fn tree(&self, _source: &marlin_agent_ui::UiSlotSource) -> Option<(&UiElement, usize)> {
        None
    }

    fn list_state(&self) -> Option<&std::cell::RefCell<super::UiListState>> {
        None
    }

    fn render(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        source: &marlin_agent_ui::UiSlotSource,
        field_template: Option<&UiElement>,
        inputs: &UiRenderInputs<'_>,
        skip: usize,
    );

    /// Render a host-provided editor at a Scheme-declared input node.
    fn input(
        &mut self,
        _frame: &mut ratatui::Frame<'_>,
        _area: Rect,
        _element: &UiElement,
        _inputs: &UiRenderInputs<'_>,
    ) {
    }

    /// Bind dynamic list data to a declarative list node.
    fn list(
        &mut self,
        _frame: &mut ratatui::Frame<'_>,
        _area: Rect,
        _element: &UiElement,
        _inputs: &UiRenderInputs<'_>,
        _skip: usize,
    ) {
    }
}

struct EmptySlots<'a>(Option<&'a std::cell::RefCell<super::UiListState>>);

impl UiSlotRenderer for EmptySlots<'_> {
    fn list_state(&self) -> Option<&std::cell::RefCell<super::UiListState>> {
        self.0
    }

    fn render(
        &mut self,
        _: &mut ratatui::Frame<'_>,
        _: Rect,
        _: &marlin_agent_ui::UiSlotSource,
        _: Option<&UiElement>,
        _: &UiRenderInputs<'_>,
        _: usize,
    ) {
    }
}

struct Slots<'a, 'tree> {
    renderer: &'a mut dyn UiSlotRenderer,
    field_template: Option<&'tree UiElement>,
}

/// Paint a tree with named facts and bindings shared by every selected branch.
pub fn render_with_facts(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    element: &UiElement,
    regions: &mut Vec<HitRegion>,
    inputs: &UiRenderInputs<'_>,
) {
    render_with_slots(frame, area, element, regions, inputs, &mut EmptySlots(None));
}

/// Paint a declarative tree while resolving host data at its slot nodes.
pub fn render_with_slots(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    element: &UiElement,
    regions: &mut Vec<HitRegion>,
    inputs: &UiRenderInputs<'_>,
    renderer: &mut dyn UiSlotRenderer,
) {
    let mut slots = Slots {
        renderer,
        field_template: None,
    };
    render_inner(frame, area, element, regions, inputs, 0, &mut slots);
}

fn render_inner<'tree>(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    element: &'tree UiElement,
    regions: &mut Vec<HitRegion>,
    inputs: &UiRenderInputs<'_>,
    skip: usize,
    slots: &mut Slots<'_, 'tree>,
) {
    let area = area.intersection(frame.area());
    if area.is_empty() {
        return;
    }
    match element {
        UiElement::Sized { child, .. } => {
            render_inner(frame, area, child, regions, inputs, skip, slots)
        }
        UiElement::List {
            source: marlin_agent_ui::UiListSource::Rows(rows),
            ..
        } => {
            super::widgets::render_bound_rows(
                frame,
                area,
                element,
                inputs,
                slots.renderer.list_state(),
                &rows[skip.min(rows.len())..],
            );
        }
        UiElement::List { .. } => slots.renderer.list(frame, area, element, inputs, skip),
        UiElement::Panel {
            border_tone,
            field_template,
            title,
            child,
        } => {
            let (title_area, inner) = panel_areas(area, border_tone.is_some(), title.is_some());
            if let Some(tone) = border_tone {
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(tone_color(*tone, inputs.theme)));
                frame.render_widget(block, area);
            }
            if let Some(title) = title {
                let title = resolve_choice(title, inputs.facts);
                if let UiElement::Template { spans, .. } = title {
                    let available = title_area.width;
                    let width = spans
                        .iter()
                        .map(|span| {
                            UnicodeWidthStr::width(
                                template_text(&span.value, inputs, available).as_ref(),
                            )
                        })
                        .sum::<usize>()
                        .min(usize::from(available)) as u16;
                    render_template(
                        frame,
                        Rect {
                            width,
                            ..title_area
                        },
                        title,
                        inputs,
                    );
                }
            }
            let previous = slots.field_template.replace(field_template);
            render_inner(frame, inner, child, regions, inputs, skip, slots);
            slots.field_template = previous;
        }
        UiElement::Input { .. } => slots.renderer.input(frame, area, element, inputs),
        UiElement::Bar { .. } => {
            if skip == 0 {
                super::widgets::render_bar(frame, area, element, regions, inputs);
            }
        }
        UiElement::Slot(id) => {
            if let Some((tree, offset)) = slots.renderer.tree(id) {
                let mut nested = Slots {
                    renderer: &mut EmptySlots(slots.renderer.list_state()),
                    field_template: None,
                };
                render_inner(
                    frame,
                    area,
                    tree,
                    regions,
                    inputs,
                    skip.saturating_add(offset),
                    &mut nested,
                );
            } else {
                slots
                    .renderer
                    .render(frame, area, id, slots.field_template, inputs, skip);
            }
        }
        UiElement::Choice { .. } => {
            render_inner(
                frame,
                area,
                resolve_choice(element, inputs.facts),
                regions,
                inputs,
                skip,
                slots,
            );
        }
        UiElement::Template { .. } => {
            if skip == 0 {
                render_template(frame, area, element, inputs)
            }
        }
        UiElement::Prompt { field, template } => {
            let value = inputs
                .bindings
                .iter()
                .find_map(|(name, value)| (*name == field.as_str()).then_some(*value))
                .unwrap_or("");
            if skip == 0
                && let Some(cursor) =
                    render_prompt(frame, area, template, inputs, field.as_str(), value)
            {
                frame.set_cursor_position(cursor);
            }
        }
        UiElement::Region {
            id,
            scroll_target,
            child,
        } => {
            regions.push(HitRegion::new(id, area, scroll_target.as_ref()));
            let scroll = if scroll_target.is_some() {
                (inputs.offset)(id)
            } else {
                0
            };
            render_inner(
                frame,
                area,
                child,
                regions,
                inputs,
                skip.saturating_add(scroll),
                slots,
            );
        }
        UiElement::Text(spans) => {
            let mut lines = vec![Line::default()];
            for span in spans {
                let style = Style::default().fg(tone_color(span.tone, inputs.theme));
                for (index, text) in span.text.split('\n').enumerate() {
                    if index > 0 {
                        lines.push(Line::default());
                    }
                    lines
                        .last_mut()
                        .expect("at least one line")
                        .spans
                        .push(Span::styled(terminal_text(text), style));
                }
            }
            frame.render_widget(
                Paragraph::new(lines).scroll((skip.min(u16::MAX as usize) as u16, 0)),
                area,
            );
        }
        UiElement::Rows(children) | UiElement::Columns(children) => {
            let vertical = matches!(element, UiElement::Rows(_));
            layout_scrolled_children(
                children,
                vertical,
                area,
                inputs.extent,
                inputs.facts,
                skip,
                |child, visible, child_skip| {
                    render_inner(frame, visible, child, regions, inputs, child_skip, slots);
                },
            );
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/org_style.rs"]
mod org_style_tests;
