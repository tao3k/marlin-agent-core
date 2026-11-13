// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Generic Ratatui mapping for admitted Scheme UI elements.

use marlin_agent_ui::{
    UiBackground, UiComponentId, UiElement, UiScrollTarget, UiSize, UiTemplateValue, UiTheme,
    UiTone,
};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub(crate) fn tone_color(tone: UiTone, theme: &UiTheme) -> Color {
    let color = match tone {
        UiTone::Normal => theme.ink,
        UiTone::Accent => theme.accent,
        UiTone::Info => theme.info,
        UiTone::Success => theme.success,
        UiTone::Warning => theme.warning,
        UiTone::Muted => theme.muted,
    };
    Color::Rgb(color.red, color.green, color.blue)
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

/// Geometry emitted by the same traversal that paints a declarative tree.
#[derive(Debug)]
pub struct HitRegion {
    pub id: UiComponentId,
    pub area: Rect,
    scroll_target: Option<HitScrollTarget>,
}

#[derive(Debug)]
enum HitScrollTarget {
    Region,
    Named(UiScrollTarget),
}

impl HitRegion {
    fn new(id: &UiComponentId, area: Rect, target: Option<&UiScrollTarget>) -> Self {
        let scroll_target = target.map(|target| {
            if target.as_str() == id.as_str() {
                HitScrollTarget::Region
            } else {
                HitScrollTarget::Named(target.clone())
            }
        });
        Self {
            id: id.clone(),
            area,
            scroll_target,
        }
    }

    /// Return the Scheme-selected destination, which may differ from the region identity.
    pub fn scroll_target(&self) -> Option<&str> {
        match self.scroll_target.as_ref()? {
            HitScrollTarget::Region => Some(self.id.as_str()),
            HitScrollTarget::Named(target) => Some(target.as_str()),
        }
    }
}

/// Space requested by an externally bound terminal component.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Extent {
    Cells(u16),
    Flex(u16),
}

#[derive(Clone, Copy, Default)]
struct Demand {
    cells: u32,
    flex: u32,
}

/// Preserve fixed and flexible demand when Scheme nests groups on one axis.
fn group_demand(
    element: &UiElement,
    vertical: bool,
    extent: &dyn Fn(&UiComponentId) -> Extent,
    facts: &dyn Fn(&UiComponentId) -> bool,
) -> Demand {
    match element {
        UiElement::Sized { size, .. } => {
            let size = match size {
                UiSize::Cells(value) => Extent::Cells(*value),
                UiSize::Flex(value) => Extent::Flex(*value),
                UiSize::Binding(name) => extent(name),
            };
            match size {
                Extent::Cells(value) => Demand {
                    cells: value.into(),
                    flex: 0,
                },
                Extent::Flex(value) => Demand {
                    cells: 0,
                    flex: value.into(),
                },
            }
        }
        UiElement::Choice { .. } => {
            group_demand(resolve_choice(element, facts), vertical, extent, facts)
        }
        UiElement::Region { child, .. } => group_demand(child, vertical, extent, facts),
        UiElement::Panel { child, .. } => group_demand(child, vertical, extent, facts),
        UiElement::Rows(children) if vertical => {
            children.iter().fold(Demand::default(), |sum, child| {
                let next = group_demand(child, vertical, extent, facts);
                Demand {
                    cells: sum.cells.saturating_add(next.cells),
                    flex: sum.flex.saturating_add(next.flex),
                }
            })
        }
        UiElement::Columns(children) if !vertical => {
            children.iter().fold(Demand::default(), |sum, child| {
                let next = group_demand(child, vertical, extent, facts);
                Demand {
                    cells: sum.cells.saturating_add(next.cells),
                    flex: sum.flex.saturating_add(next.flex),
                }
            })
        }
        _ if vertical => Demand {
            cells: element.intrinsic_rows_with(facts).min(u16::MAX as usize) as u32,
            flex: 0,
        },
        _ => Demand { cells: 0, flex: 1 },
    }
}

fn layout_children(
    children: &[UiElement],
    vertical: bool,
    area: Rect,
    extent: &dyn Fn(&UiComponentId) -> Extent,
    facts: &dyn Fn(&UiComponentId) -> bool,
    mut visit: impl FnMut(&UiElement, Rect),
) {
    let limit = if vertical { area.height } else { area.width };
    // A single child is common for compositional wrappers; no sizing vector is needed.
    if let [child] = children {
        let demand = group_demand(child, vertical, extent, facts);
        let extra = if demand.flex > 0 {
            u32::from(limit).saturating_sub(demand.cells)
        } else {
            0
        };
        let length = demand.cells.saturating_add(extra).min(u32::from(limit)) as u16;
        let rect = if vertical {
            Rect::new(area.x, area.y, area.width, length)
        } else {
            Rect::new(area.x, area.y, length, area.height)
        };
        visit(child, rect);
        return;
    }
    let sizing: Vec<_> = children
        .iter()
        .map(|child| group_demand(child, vertical, extent, facts))
        .collect();
    let fixed = sizing
        .iter()
        .fold(0u32, |sum, size| sum.saturating_add(size.cells));
    let mut available = u32::from(limit).saturating_sub(fixed);
    let mut weight = sizing
        .iter()
        .fold(0u32, |sum, size| sum.saturating_add(size.flex));
    let mut cursor = 0u16;
    for (child, size) in children.iter().zip(sizing) {
        let extra = if size.flex > 0 && weight > 0 {
            let portion = (u64::from(available) * u64::from(size.flex) / u64::from(weight)) as u32;
            available -= portion;
            weight -= size.flex;
            portion
        } else {
            0
        };
        let length = size
            .cells
            .saturating_add(extra)
            .min(u32::from(limit.saturating_sub(cursor))) as u16;
        let rect = if vertical {
            Rect::new(area.x, area.y.saturating_add(cursor), area.width, length)
        } else {
            Rect::new(area.x.saturating_add(cursor), area.y, length, area.height)
        };
        cursor = cursor.saturating_add(length);
        visit(child, rect);
    }
}

/// Map a declarative region tree to terminal rectangles without painting it.
///
/// `regions` is appended in traversal order; callers clear it for each layout.
pub fn layout_regions(
    element: &UiElement,
    area: Rect,
    extent: &dyn Fn(&UiComponentId) -> Extent,
    regions: &mut Vec<HitRegion>,
) {
    layout_regions_with_facts(element, area, extent, regions, &|_| false);
}

/// Lay out the selected branches using the same facts as painting.
pub fn layout_regions_with_facts(
    element: &UiElement,
    area: Rect,
    extent: &dyn Fn(&UiComponentId) -> Extent,
    regions: &mut Vec<HitRegion>,
    facts: &dyn Fn(&UiComponentId) -> bool,
) {
    if area.is_empty() {
        return;
    }
    match element {
        UiElement::Region {
            id,
            scroll_target,
            child,
        } => {
            regions.push(HitRegion::new(id, area, scroll_target.as_ref()));
            layout_regions_with_facts(child, area, extent, regions, facts);
        }
        UiElement::Text(_)
        | UiElement::Slot(_)
        | UiElement::Template { .. }
        | UiElement::Input { .. } => {}
        UiElement::Choice { .. } => {
            layout_regions_with_facts(resolve_choice(element, facts), area, extent, regions, facts);
        }
        UiElement::Sized { child, .. } => {
            layout_regions_with_facts(child, area, extent, regions, facts);
        }
        UiElement::Panel { child, .. } => {
            let inner = Block::default().borders(Borders::ALL).inner(area);
            layout_regions_with_facts(child, inner, extent, regions, facts);
        }
        UiElement::Rows(children) | UiElement::Columns(children) => {
            let vertical = matches!(element, UiElement::Rows(_));
            layout_children(children, vertical, area, extent, facts, |child, rect| {
                layout_regions_with_facts(child, rect, extent, regions, facts);
            });
        }
    }
}

/// Render a Scheme-declared line with typed, named host values.
pub fn render_template(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    element: &UiElement,
    theme: &UiTheme,
    bindings: &[(&str, &str)],
) {
    let UiElement::Template { background, spans } = element else {
        return;
    };
    let area = area.intersection(frame.area());
    if area.is_empty() {
        return;
    }
    let line: Vec<_> = spans
        .iter()
        .map(|span| {
            let value = match &span.value {
                UiTemplateValue::Literal(value) => value.as_str(),
                UiTemplateValue::Binding(id) => bindings
                    .iter()
                    .find_map(|(name, value)| (*name == id.as_str()).then_some(*value))
                    .unwrap_or(""),
            };
            let text = terminal_text(value);
            let mut style = Style::default().fg(tone_color(span.tone, theme));
            if span.bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            Span::styled(text, style)
        })
        .collect();
    let background = match background {
        UiBackground::None => None,
        UiBackground::Canvas => Some(Color::Rgb(
            theme.canvas.red,
            theme.canvas.green,
            theme.canvas.blue,
        )),
        UiBackground::Bar => Some(Color::Rgb(theme.bar.red, theme.bar.green, theme.bar.blue)),
        UiBackground::Tone(tone) => Some(tone_color(*tone, theme)),
    };
    let style = background.map_or(Style::default(), |color| Style::default().bg(color));
    frame.render_widget(Paragraph::new(Line::from(line)).style(style), area);
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
        .map(|span| {
            let text = match &span.value {
                UiTemplateValue::Literal(value) => value.as_str(),
                UiTemplateValue::Binding(id) => inputs
                    .bindings
                    .iter()
                    .find_map(|(name, value)| (*name == id.as_str()).then_some(*value))
                    .unwrap_or(""),
            };
            UnicodeWidthStr::width(terminal_text(text).as_ref())
        })
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
    let mut bindings = inputs.bindings.to_vec();
    bindings.push((field, visible));
    render_template(frame, area, selected, inputs.theme, &bindings);
    let x = (prefix_width + visible_width).min(usize::from(area.width - 1));
    Some((area.x + x as u16, area.y))
}

/// Find the innermost painted region containing a terminal cell.
pub fn hit_test(regions: &[HitRegion], x: u16, y: u16) -> Option<&HitRegion> {
    regions.iter().rev().find(|region| {
        x >= region.area.x
            && x < region.area.right()
            && y >= region.area.y
            && y < region.area.bottom()
    })
}

/// Find the innermost scrollable ancestor at a terminal cell.
pub fn hit_test_scroll(regions: &[HitRegion], x: u16, y: u16) -> Option<&HitRegion> {
    regions.iter().rev().find(|region| {
        region.scroll_target.is_some()
            && x >= region.area.x
            && x < region.area.right()
            && y >= region.area.y
            && y < region.area.bottom()
    })
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
    fn title(&self, _id: &UiComponentId) -> Option<&str> {
        None
    }

    fn render(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        id: &UiComponentId,
        field_tone: Option<UiTone>,
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
}

struct EmptySlots;

impl UiSlotRenderer for EmptySlots {
    fn render(
        &mut self,
        _: &mut ratatui::Frame<'_>,
        _: Rect,
        _: &UiComponentId,
        _: Option<UiTone>,
        _: usize,
    ) {
    }
}

struct Slots<'a> {
    renderer: &'a mut dyn UiSlotRenderer,
    field_tone: Option<UiTone>,
}

/// Paint a tree with named facts and bindings shared by every selected branch.
pub fn render_with_facts(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    element: &UiElement,
    regions: &mut Vec<HitRegion>,
    inputs: &UiRenderInputs<'_>,
) {
    render_with_slots(frame, area, element, regions, inputs, &mut EmptySlots);
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
        field_tone: None,
    };
    render_inner(frame, area, element, regions, inputs, 0, &mut slots);
}

fn render_inner(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    element: &UiElement,
    regions: &mut Vec<HitRegion>,
    inputs: &UiRenderInputs<'_>,
    skip: usize,
    slots: &mut Slots<'_>,
) {
    let area = area.intersection(frame.area());
    if area.is_empty() {
        return;
    }
    match element {
        UiElement::Sized { child, .. } => {
            render_inner(frame, area, child, regions, inputs, skip, slots)
        }
        UiElement::Panel {
            border_tone,
            field_tone,
            child,
        } => {
            let mut block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(tone_color(*border_tone, inputs.theme)));
            if let UiElement::Slot(id) = resolve_choice(child, inputs.facts)
                && let Some(title) = slots.renderer.title(id)
            {
                let title: String = title
                    .chars()
                    .take(128)
                    .map(|c| if c.is_control() { '�' } else { c })
                    .collect();
                block = block.title(format!(" {title} "));
            }
            let inner = block.inner(area);
            frame.render_widget(block, area);
            let previous = slots.field_tone.replace(*field_tone);
            render_inner(frame, inner, child, regions, inputs, skip, slots);
            slots.field_tone = previous;
        }
        UiElement::Input { .. } => slots.renderer.input(frame, area, element, inputs),
        UiElement::Slot(id) => slots
            .renderer
            .render(frame, area, id, slots.field_tone, skip),
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
                render_template(frame, area, element, inputs.theme, inputs.bindings)
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
                        .push(Span::styled(text, style));
                }
            }
            frame.render_widget(
                Paragraph::new(lines).scroll((skip.min(u16::MAX as usize) as u16, 0)),
                area,
            );
        }
        UiElement::Rows(children) | UiElement::Columns(children) => {
            if children.is_empty() {
                return;
            }
            let vertical = matches!(element, UiElement::Rows(_));
            let height = if vertical && skip > 0 {
                let fixed = children.iter().fold(0u32, |total, child| {
                    total.saturating_add(
                        group_demand(child, true, inputs.extent, inputs.facts).cells,
                    )
                });
                area.height.max(fixed.min(u32::from(u16::MAX)) as u16)
            } else {
                area.height
            };
            let virtual_area = Rect::new(0, 0, area.width, height);
            layout_children(
                children,
                vertical,
                virtual_area,
                inputs.extent,
                inputs.facts,
                |child, rect| {
                    if vertical {
                        let start = usize::from(rect.y);
                        let visible_start = start.max(skip);
                        let visible_end = usize::from(rect.bottom())
                            .min(skip.saturating_add(usize::from(area.height)));
                        if visible_start < visible_end {
                            let visible = Rect::new(
                                area.x,
                                area.y + (visible_start - skip) as u16,
                                area.width,
                                (visible_end - visible_start) as u16,
                            );
                            render_inner(
                                frame,
                                visible,
                                child,
                                regions,
                                inputs,
                                visible_start - start,
                                slots,
                            );
                        }
                    } else {
                        let visible = Rect::new(
                            area.x.saturating_add(rect.x),
                            area.y,
                            rect.width,
                            area.height,
                        );
                        render_inner(frame, visible, child, regions, inputs, skip, slots);
                    }
                },
            );
        }
    }
}
