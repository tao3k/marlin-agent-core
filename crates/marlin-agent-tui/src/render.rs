// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Terminal frame projection for the active conversation or workspace view.

#[path = "render_main.rs"]
mod main_view;

use super::{
    app::{App, BufferView, GatewayObservation, ModelineActivity},
    composer::Composer,
    ui_elements,
    ui_host::MainSource,
    viewport::{ScrollAnchor, TranscriptViewportCache},
};
use marlin_agent_ui::{UiLayoutPolicy, UiRegion, UiRgb, UiWorkbench};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
};
use unicode_width::UnicodeWidthStr;

// Configuration and session identifiers may contain control characters. Keep
// the header bounded and prevent them from becoming terminal control input.
fn display_value(value: &str, limit: usize) -> String {
    value
        .chars()
        .take(limit)
        .map(|ch| if ch.is_control() { '�' } else { ch })
        .collect()
}

fn header_value(value: &str) -> String {
    display_value(value, 128)
}

fn menu_display_name<'a>(name: &'a str, prefix: Option<&str>) -> &'a str {
    prefix
        .and_then(|prefix| name.strip_prefix(prefix))
        .unwrap_or(name)
}

fn rgb(color: UiRgb) -> Color {
    Color::Rgb(color.red, color.green, color.blue)
}

pub(crate) fn editor_fact(app: &App, policy: &UiLayoutPolicy, name: &str) -> bool {
    match name {
        "command" => app.command_draft.is_some() || app.tool_review_prompt.is_some(),
        "editor-fixed" => {
            let frame = if app.command_draft.is_some() || app.tool_review_prompt.is_some() {
                &policy.command_frame
            } else {
                &policy.message_frame
            };
            frame.fixed_editor_rows > 0
        }
        "editor-expanded" => {
            app.minibuffer_hovered
                || !app.composer.text().is_empty()
                || app.history_search.is_some()
        }
        _ => false,
    }
}

// Layout and painting must choose the same Scheme branch for a given frame.
fn main_view_fact(
    app: &App,
    ui: &UiWorkbench,
    buffer: &BufferView,
    previewing: bool,
    name: &str,
) -> bool {
    match name {
        "review" => app.tool_review_prompt.is_some(),
        "messages" => *buffer == BufferView::Messages,
        "static" => match buffer {
            BufferView::Named(buffer) => ui.regions.iter().any(|region| {
                matches!(region, UiRegion::Extension { id, focusable: false, .. }
                    if id.as_str() == buffer.as_str())
            }),
            _ => false,
        },
        "runtime" => !previewing && app.runtime_receipt_open,
        "extension" => {
            !previewing
                && *buffer == BufferView::Conversation
                && app.workspace.view().is_none()
                && !app.runtime_receipt_open
                && ui.regions.iter().any(|region| {
                    matches!(region, UiRegion::Extension { id, focusable: true, .. }
                        if id == &ui.focus)
                })
        }
        "workspace" => !previewing && app.workspace.view().is_some(),
        "modeline-present" => ui
            .regions
            .iter()
            .any(|region| matches!(region, UiRegion::Status(items) if !items.is_empty())),
        "command" | "editor-fixed" | "editor-expanded" => editor_fact(app, &ui.layout, name),
        "extensions-enabled" => {
            if app.command_draft.is_some() || app.tool_review_prompt.is_some() {
                ui.layout.command_frame.extensions
            } else {
                ui.layout.message_frame.extensions
            }
        }
        "focused" => ui.focus.as_str() == "composer" || app.command_draft.is_some(),
        _ => false,
    }
}

fn activity_fact(app: &App) -> &'static str {
    match app.modeline_activity() {
        ModelineActivity::Idle => match app.gateway_observation {
            GatewayObservation::Unchecked => "",
            GatewayObservation::Online => "activity-online",
            GatewayObservation::Offline => "activity-offline",
            GatewayObservation::Unknown => "activity-unknown",
        },
        ModelineActivity::Runtime => "activity-runtime",
        ModelineActivity::Workspace => "activity-workspace",
        ModelineActivity::Recovery => "activity-recovery",
        ModelineActivity::ContinuationUnavailable => "activity-continuation",
        ModelineActivity::History => "activity-history",
        ModelineActivity::Working => "activity-working",
        ModelineActivity::ReadingWorkspace => "activity-reading",
    }
}

fn frame_fact(
    app: &App,
    ui: &UiWorkbench,
    buffer: &BufferView,
    previewing: bool,
    collapsed: bool,
    extra_count: usize,
    name: &str,
) -> bool {
    match name {
        "searching" => app
            .history_search
            .as_ref()
            .is_some_and(|search| search.is_searching()),
        "search-match" => app
            .history_search
            .as_ref()
            .is_some_and(|search| search.preview().is_some()),
        "search-no-match" => app.history_search.is_some(),
        "notice" => app.minibuffer_notice.is_some(),
        "static-buffer" => main_view_fact(app, ui, buffer, previewing, "static"),
        "editor-collapsed" => collapsed,
        "extra-one" => extra_count >= 1,
        "extra-two" => extra_count >= 2,
        "quit-armed" => app.quit_armed,
        "draft-limit" => app.composer.limit_reached(),
        "commands-focus" => ui.focus.as_str() == "commands",
        "transcript-focus" => ui.focus.as_str() == "transcript",
        "extension-focus" => ui.focus.as_str() != "composer",
        "normal-mode" => {
            app.command_draft.is_none()
                && app.tool_review_prompt.is_none()
                && app.key_mode == marlin_agent_ui::UiKeyMode::Normal
        }
        "has-turns" => app.history.len() >= 2,
        name if name.starts_with("activity-") => name == activity_fact(app),
        name => main_view_fact(app, ui, buffer, previewing, name),
    }
}

pub(crate) struct EditorMeasurements<'a> {
    fixed: u16,
    measured: std::cell::Cell<Option<u16>>,
    footer_min: u16,
    hint_rows: u16,
    transcript_min: u16,
    height: u16,
    composer: &'a Composer,
    width: u16,
    border: u16,
    max_rows: u16,
}

impl EditorMeasurements<'_> {
    pub(crate) fn extent(&self, id: &marlin_agent_ui::UiComponentId) -> super::ui_elements::Extent {
        use super::ui_elements::Extent;
        Extent::Cells(match id.as_str() {
            "fixed-editor" => self.fixed,
            "measured-editor" => match self.measured.get() {
                Some(value) => value,
                None => {
                    let value = self
                        .composer
                        .preferred_rows(
                            self.width.saturating_sub(self.border),
                            self.max_rows.saturating_sub(self.border),
                        )
                        .saturating_add(self.border);
                    self.measured.set(Some(value));
                    value
                }
            },
            "footer-min" => self.footer_min,
            "hint-rows" => self.hint_rows,
            "transcript-min" => self.transcript_min,
            "chatbox-limit" => self.max_rows,
            "terminal-height" => self.height,
            _ => 0,
        })
    }
}

pub(crate) fn editor_measurements<'a>(
    app: &'a App,
    policy: &UiLayoutPolicy,
    width: u16,
    height: u16,
    facts: &dyn Fn(&marlin_agent_ui::UiComponentId) -> bool,
) -> EditorMeasurements<'a> {
    let frame = if app.command_draft.is_some() || app.tool_review_prompt.is_some() {
        &policy.command_frame
    } else {
        &policy.message_frame
    };
    let composer = app
        .history_search
        .as_ref()
        .and_then(|search| search.preview())
        .unwrap_or(&app.composer);
    let border = policy
        .frame_tree
        .region_child("editor")
        .and_then(|editor| super::ui_elements::input_border(editor, facts))
        .unwrap_or(0);
    EditorMeasurements {
        fixed: frame.fixed_editor_rows,
        measured: std::cell::Cell::new(None),
        footer_min: frame.footer_rows,
        hint_rows: app.transient_hint.as_ref().map_or(0, |hint| {
            u16::try_from(hint.intrinsic_rows()).unwrap_or(u16::MAX)
        }),
        transcript_min: frame.min_transcript_rows,
        height,
        composer,
        width,
        border,
        max_rows: policy.max_chatbox_rows,
    }
}

#[cfg(test)]
pub(crate) fn frame_regions(
    app: &App,
    ui: &UiWorkbench,
    area: Rect,
) -> Vec<super::ui_elements::HitRegion> {
    frame_snapshot(app, ui, area).regions
}

pub(crate) struct FrameSnapshot {
    pub(crate) regions: Vec<super::ui_elements::HitRegion>,
    pub(crate) area: Rect,
    collapsed: bool,
    extras: Vec<String>,
}

pub(crate) fn frame_snapshot(app: &App, ui: &UiWorkbench, area: Rect) -> FrameSnapshot {
    use super::ui_elements::{Extent, RegionExtents, layout_regions_with_facts};
    let preview = app.preview_buffer();
    let buffer = preview.as_ref().unwrap_or(&app.buffer_view);
    let suggestions = if app.history_search.is_none()
        && app.command_draft.is_some()
        && app.tool_review_prompt.is_none()
    {
        app.command_candidates.as_slice()
    } else {
        &[]
    };
    let prefix = app.command_menu.as_ref().map(|menu| menu.prefix.as_str());
    let name_width = suggestions
        .iter()
        .map(|name| UnicodeWidthStr::width(menu_display_name(name.as_str(), prefix)))
        .max()
        .unwrap_or(0);
    let mut collapsed = !editor_fact(app, &ui.layout, "editor-fixed")
        && !editor_fact(app, &ui.layout, "editor-expanded");
    // Derived editor state normally stabilizes immediately. If an authored
    // layout oscillates, cap evaluation and retain one coherent fact snapshot.
    for attempt in 0..3 {
        let extras: Vec<_> = ui
            .regions
            .iter()
            .filter_map(|region| match region {
                UiRegion::Status(items) if collapsed => Some(items),
                _ => None,
            })
            .flatten()
            .filter(|item| !item.id.as_str().starts_with("modeline/"))
            .take(2)
            .map(|item| display_value(&item.label, 16))
            .collect();
        let facts = |id: &marlin_agent_ui::UiComponentId| {
            frame_fact(
                app,
                ui,
                buffer,
                preview.is_some(),
                collapsed,
                extras.len(),
                id.as_str(),
            )
        };
        let metrics = FrameMeasurements::new(app, ui, area, &facts);
        let raw_extent = |id: &marlin_agent_ui::UiComponentId| match id.as_str() {
            "extra-one-width" | "extra-two-width" => {
                let index = usize::from(id.as_str() == "extra-two-width");
                Extent::Cells(extras.get(index).map_or(0, |text| {
                    u16::try_from(UnicodeWidthStr::width(text.as_str())).unwrap_or(u16::MAX)
                }))
            }
            "candidate-name-width" => Extent::Cells(u16::try_from(name_width).unwrap_or(u16::MAX)),
            _ => metrics.extent(id),
        };
        let extents = RegionExtents::new(&ui.layout.frame_tree, &raw_extent, &facts);
        let mut regions = Vec::with_capacity(7);
        layout_regions_with_facts(
            &ui.layout.frame_tree,
            area,
            &|id| extents.extent(id),
            &mut regions,
            &facts,
        );
        let actual = regions
            .iter()
            .find(|region| region.id.as_str() == "editor")
            .is_none_or(|region| region.area.height == 0);
        if actual == collapsed || attempt == 2 {
            return FrameSnapshot {
                regions,
                area,
                collapsed,
                extras,
            };
        }
        collapsed = actual;
    }
    unreachable!("bounded frame snapshot loop")
}

struct FrameMeasurements<'a> {
    editor: EditorMeasurements<'a>,
    review_count: u16,
    command_count: u16,
    command_limit: u16,
    extension_count: u16,
}

impl<'a> FrameMeasurements<'a> {
    fn new(
        app: &'a App,
        ui: &'a UiWorkbench,
        area: Rect,
        facts: &dyn Fn(&marlin_agent_ui::UiComponentId) -> bool,
    ) -> Self {
        let review_count = app.tool_review_prompt.as_ref().map_or(0, |prompt| {
            u16::try_from(prompt.menu.choices.len()).unwrap_or(u16::MAX)
        });
        let extension_count = u16::try_from(
            ui.regions
                .iter()
                .filter(|region| {
                    matches!(
                        region,
                        UiRegion::Extension {
                            focusable: true,
                            ..
                        }
                    )
                })
                .count(),
        )
        .unwrap_or(u16::MAX);
        Self {
            editor: editor_measurements(app, &ui.layout, area.width, area.height, facts),
            review_count,
            command_count: u16::try_from(app.command_candidates.len()).unwrap_or(u16::MAX),
            command_limit: ui.layout.max_command_rows,
            extension_count,
        }
    }

    fn extent(&self, id: &marlin_agent_ui::UiComponentId) -> super::ui_elements::Extent {
        use super::ui_elements::Extent;
        match id.as_str() {
            "review-count" => Extent::Cells(self.review_count),
            "command-count" => Extent::Cells(self.command_count),
            "command-limit" => Extent::Cells(self.command_limit),
            "extension-count" => Extent::Cells(self.extension_count),
            _ => self.editor.extent(id),
        }
    }
}

pub(crate) fn mouse_over_minibuffer(
    regions: &[super::ui_elements::HitRegion],
    column: u16,
    row: u16,
) -> bool {
    column > 0
        && row > 0
        && regions.iter().any(|region| {
            region.id.as_str() == "minibuffer-hover"
                && column > region.area.x
                && column - 1 < region.area.right()
                && row > region.area.y
                && row - 1 < region.area.bottom()
        })
}

#[cfg(test)]
pub(crate) fn draw(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    ui: &UiWorkbench,
    cache: &mut TranscriptViewportCache,
    anchor: &mut ScrollAnchor,
) -> usize {
    let snapshot = frame_snapshot(app, ui, frame.area());
    draw_with_snapshot(frame, app, ui, cache, anchor, &snapshot)
}

pub(crate) fn draw_with_snapshot(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    ui: &UiWorkbench,
    cache: &mut TranscriptViewportCache,
    anchor: &mut ScrollAnchor,
    snapshot: &FrameSnapshot,
) -> usize {
    use super::ui_elements::{Extent, RegionExtents, UiRenderInputs};

    app.list_state.borrow_mut().begin_frame();
    let area = frame.area();
    frame.buffer_mut().set_style(
        area,
        Style::default()
            .bg(rgb(ui.theme.canvas))
            .fg(rgb(ui.theme.ink)),
    );
    let command_active = app.command_draft.is_some() || app.tool_review_prompt.is_some();
    let frame_spec = if command_active {
        &ui.layout.command_frame
    } else {
        &ui.layout.message_frame
    };
    let composer = app
        .history_search
        .as_ref()
        .and_then(|search| search.preview())
        .unwrap_or(&app.composer);
    let extensions: Vec<_> = ui
        .regions
        .iter()
        .filter_map(|region| match region {
            UiRegion::Extension {
                id,
                label,
                focusable: true,
                ..
            } => Some((id, label.as_str())),
            _ => None,
        })
        .collect();
    let suggestions = if app.history_search.is_none()
        && app.command_draft.is_some()
        && app.tool_review_prompt.is_none()
    {
        app.command_candidates.as_slice()
    } else {
        &[]
    };
    let regions = &snapshot.regions;
    let part = |name: &str| {
        regions
            .iter()
            .find(|region| region.id.as_str() == name)
            .map_or(Rect::default(), |region| region.area)
    };
    let editor_area = part("editor");
    let minibuffer_area = part("minibuffer");
    let preview = app.preview_buffer();
    let buffer = preview.as_ref().unwrap_or(&app.buffer_view);
    let focused_content = if preview.is_none()
        && *buffer == BufferView::Conversation
        && app.workspace.view().is_none()
        && !app.runtime_receipt_open
    {
        ui.regions.iter().find_map(|region| match region {
            UiRegion::Extension {
                id,
                label,
                content,
                focusable: true,
            } if id == &ui.focus => Some((label.as_str(), content.as_slice())),
            _ => None,
        })
    } else {
        None
    };
    let static_content = match buffer {
        BufferView::Named(buffer) => ui.regions.iter().find_map(|region| match region {
            UiRegion::Extension {
                id,
                label,
                content,
                focusable: false,
            } if id.as_str() == buffer.as_str() => Some((
                label.as_str(),
                app.dynamic_buffer_content
                    .get(buffer)
                    .map_or(content.as_slice(), Vec::as_slice),
            )),
            _ => None,
        }),
        _ => None,
    };
    let transcript_policy = &ui.layout.transcript_policy;
    let mut role_style = Style::default().fg(super::ui_elements::tone_color(
        transcript_policy.role_tone,
        &ui.theme,
    ));
    if transcript_policy.role_bold {
        role_style = role_style.add_modifier(Modifier::BOLD);
    }
    let selected = app
        .selected_command()
        .filter(|name| suggestions.contains(name));
    let active = selected.or_else(|| suggestions.first());
    let selected_index = app
        .tool_review_prompt
        .as_ref()
        .map(|prompt| prompt.selected)
        .or_else(|| active.and_then(|name| suggestions.iter().position(|item| item == name)));
    let command_prompt = app.tool_review_prompt.as_ref().map_or_else(
        || {
            app.command_menu
                .as_ref()
                .map_or(frame_spec.prompt.as_str(), |menu| menu.prompt.as_str())
        },
        |prompt| prompt.menu.prompt.as_str(),
    );
    let title = if app.history_search.is_some() {
        " History · Enter accept · Esc cancel "
    } else {
        command_prompt
    };
    let selected_count = selected_index.map_or(0, |index| index + 1).to_string();
    let candidate_count = app
        .tool_review_prompt
        .as_ref()
        .map_or(suggestions.len(), |prompt| prompt.menu.choices.len())
        .to_string();
    let menu_prefix = app.command_menu.as_ref().map(|menu| menu.prefix.as_str());
    let name_width = suggestions
        .iter()
        .map(|name| UnicodeWidthStr::width(menu_display_name(name.as_str(), menu_prefix)))
        .max()
        .unwrap_or(0);
    let model = header_value(&ui.model_label);
    let session = header_value(&ui.session_label);
    let turns = (app.history.len() / 2).to_string();
    let static_name = static_content.map(|(label, _)| header_value(label));
    let panel_title = static_content
        .or(focused_content)
        .map_or_else(String::new, |(title, _)| header_value(title));
    let extra_one = snapshot.extras.first();
    let extra_two = snapshot.extras.get(1);
    let facts = |id: &marlin_agent_ui::UiComponentId| {
        frame_fact(
            app,
            ui,
            buffer,
            preview.is_some(),
            snapshot.collapsed,
            snapshot.extras.len(),
            id.as_str(),
        )
    };
    let measurements = FrameMeasurements::new(app, ui, area, &facts);
    let raw_extent = |id: &marlin_agent_ui::UiComponentId| match id.as_str() {
        "extra-one-width" | "extra-two-width" => {
            let value = if id.as_str() == "extra-one-width" {
                extra_one.map(String::as_str)
            } else {
                extra_two.map(String::as_str)
            };
            Extent::Cells(value.map_or(0, |text| {
                u16::try_from(UnicodeWidthStr::width(text)).unwrap_or(u16::MAX)
            }))
        }
        "candidate-name-width" => Extent::Cells(u16::try_from(name_width).unwrap_or(u16::MAX)),
        _ => measurements.extent(id),
    };
    let extents = RegionExtents::new(&ui.layout.frame_tree, &raw_extent, &facts);
    let bindings = [
        ("panel-title", panel_title.as_str()),
        ("selected-count", selected_count.as_str()),
        ("candidate-count", candidate_count.as_str()),
        ("prompt", command_prompt.trim_start()),
        ("compact-prompt", command_prompt),
        ("title", title),
        ("model", model.as_str()),
        ("session", session.as_str()),
        ("turns", turns.as_str()),
        ("extra-one", extra_one.map_or("", String::as_str)),
        ("extra-two", extra_two.map_or("", String::as_str)),
        (
            "notice",
            app.minibuffer_notice.as_deref().unwrap_or("").trim_start(),
        ),
        ("static-name", static_name.as_deref().unwrap_or("")),
    ];
    let inputs = UiRenderInputs {
        theme: &ui.theme,
        offset: &|_| 0,
        facts: &facts,
        extent: &|id| extents.extent(id),
        bindings: &bindings,
    };
    let mut slots = main_view::FrameSlots {
        main: main_view::MainSlots {
            app,
            cache,
            anchor,
            role_style,
            static_content,
            focused_content,
            effective_scroll: app.scroll_up,
        },
        editor: main_view::EditorSlots {
            composer,
            show_cursor: command_active
                || (app.history_search.is_none() && ui.focus.as_str() == "composer"),
            read_only: app.tool_review_prompt.is_some(),
        },
        app,
        ui,
        suggestions,
        extensions: &extensions,
        selected: selected_index,
        menu_prefix,
    };
    super::ui_elements::render_with_slots(
        frame,
        area,
        &ui.layout.frame_tree,
        &mut Vec::new(),
        &inputs,
        &mut slots,
    );
    if editor_area.height == 0
        && ui.focus.as_str() == "composer"
        && minibuffer_area.width > 2
        && app.transient_hint.is_none()
        && app.history_search.is_none()
    {
        frame.set_cursor_position((minibuffer_area.x + 2, minibuffer_area.y));
    }
    slots.main.effective_scroll
}
