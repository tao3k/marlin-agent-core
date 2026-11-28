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
    viewport::{ScrollAnchor, TranscriptViewportCache},
};
use marlin_agent_ui::{UiComponentId, UiFrameRole, UiLayoutPolicy, UiRegion, UiRgb, UiWorkbench};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
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
            app.editor_keyboard_active
                || (app.minibuffer_hovered && app.key_mode == marlin_agent_ui::UiKeyMode::Insert)
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

fn echo_bindings(app: &App) -> [(&'static str, &str); 2] {
    [
        (
            "key-echo",
            app.echo_area
                .keys
                .as_ref()
                .map_or("", |echo| echo.text.as_str()),
        ),
        ("notice", app.echo_area.message().map_or("", str::trim_start)),
    ]
}

struct FrameFacts<'a> {
    extra_count: usize,
    bindings: &'a [(UiComponentId, String)],
}

fn frame_fact(
    app: &App,
    ui: &UiWorkbench,
    buffer: &BufferView,
    previewing: bool,
    frame: &FrameFacts<'_>,
    name: &str,
) -> bool {
    if let Some(names) = name.strip_prefix("any/") {
        return names
            .split('/')
            .any(|name| frame_fact(app, ui, buffer, previewing, frame, name));
    }
    if let Some(id) = name.strip_prefix("binding-present/") {
        return frame
            .bindings
            .iter()
            .find_map(|(name, value)| (name.as_str() == id).then_some(!value.is_empty()))
            .unwrap_or_else(|| {
                echo_bindings(app)
                    .iter()
                    .any(|(name, value)| *name == id && !value.is_empty())
            });
    }
    let comparison = name
        .strip_prefix("binding-is/")
        .map(|rest| (true, rest))
        .or_else(|| {
            name.strip_prefix("binding-is-not/")
                .map(|rest| (false, rest))
        });
    if let Some((equal, comparison)) = comparison {
        return comparison
            .split_once('/')
            .is_some_and(|(binding, expected)| {
                let value = match binding {
                    "active-buffer" => buffer.as_str(),
                    "focus" => ui.focus.as_str(),
                    _ => return false,
                };
                (value == expected) == equal
            });
    }
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
        "transient" => app.transient_hint.is_some(),
        "keypad" => app.keypad_open,
        "key-prefix-pending" => app
            .echo_area
            .keys
            .as_ref()
            .is_some_and(|echo| echo.state == crate::ui_host::KeySequenceState::Pending),
        "key-undefined" => app
            .echo_area
            .keys
            .as_ref()
            .is_some_and(|echo| echo.state == crate::ui_host::KeySequenceState::Undefined),
        "static-buffer" => main_view_fact(app, ui, buffer, previewing, "static"),
        "extra-one" => frame.extra_count >= 1,
        "extra-two" => frame.extra_count >= 2,
        "quit-armed" => app.quit_armed,
        "draft-limit" => app.composer.limit_reached(),
        "commands-focus" => ui.focus.as_str() == "commands",
        "transcript-focus" => ui.focus.as_str() == "transcript",
        "extension-focus" => ui.focus.as_str() != "composer",
        "normal-mode" => app.key_mode == marlin_agent_ui::UiKeyMode::Normal,
        "has-turns" => app.history.len() >= 2,
        name if name.starts_with("activity-") => name == activity_fact(app),
        name => main_view_fact(app, ui, buffer, previewing, name),
    }
}

fn echo_text(
    app: &App,
    ui: &UiWorkbench,
    area: Rect,
    buffer: &BufferView,
    previewing: bool,
    bindings: &[(UiComponentId, String)],
) -> String {
    let frame = FrameFacts {
        extra_count: 0,
        bindings,
    };
    let facts = |id: &UiComponentId| {
        ui_elements::fits_area(id, area)
            .unwrap_or_else(|| frame_fact(app, ui, buffer, previewing, &frame, id.as_str()))
    };
    let raw = echo_bindings(app);
    let mut provided = Vec::new();
    let values = if bindings.is_empty() {
        raw.as_slice()
    } else {
        provided.extend(
            bindings
                .iter()
                .map(|(id, value)| (id.as_str(), value.as_str())),
        );
        provided.extend(raw);
        provided.as_slice()
    };
    let inputs = ui_elements::UiRenderInputs {
        theme: &ui.theme,
        facts: &facts,
        bindings: values,
        extent: &|_| ui_elements::Extent::Cells(0),
        offset: &|_| 0,
    };
    ui_elements::inline_text(&ui.layout.echo.view, &inputs, area.width)
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
    insets: super::ui_elements::InputInsets,
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
                            self.insets.text_width(self.width, true),
                            self.max_rows.saturating_sub(self.insets.border),
                        )
                        .saturating_add(self.insets.border);
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
    let insets = policy
        .frame_tree
        .region_child(policy.regions.id(UiFrameRole::Editor).as_str())
        .and_then(|editor| super::ui_elements::input_insets(editor, facts))
        .map_or_else(Default::default, |insets| insets);
    EditorMeasurements {
        fixed: frame.fixed_editor_rows,
        measured: std::cell::Cell::new(None),
        footer_min: frame.footer_rows,
        hint_rows: app.transient_hint.as_ref().map_or(0, |hint| {
            u16::try_from(hint.intrinsic_rows_with(facts)).unwrap_or(u16::MAX)
        }),
        transcript_min: frame.min_transcript_rows,
        height,
        composer,
        width,
        insets,
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
    pub(crate) editor_width: u16,
    extras: Vec<String>,
    bindings: Vec<(UiComponentId, String)>,
}

pub(crate) fn frame_snapshot(app: &App, ui: &UiWorkbench, area: Rect) -> FrameSnapshot {
    frame_snapshot_with_bindings(app, ui, area, Vec::new())
}

pub(crate) fn frame_snapshot_with_bindings(
    app: &App,
    ui: &UiWorkbench,
    area: Rect,
    mut bindings: Vec<(UiComponentId, String)>,
) -> FrameSnapshot {
    use super::ui_elements::{Extent, RegionExtents, layout_regions_with_facts};
    let preview = app.preview_buffer();
    let buffer = preview.as_ref().unwrap_or(&app.buffer_view);
    let echo = echo_text(app, ui, area, buffer, preview.is_some(), &bindings);
    bindings.retain(|(id, _)| id.as_str() != "echo");
    if !echo.is_empty() {
        bindings.push((
            UiComponentId::new("echo").expect("static frame binding"),
            echo,
        ));
    }
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
        let frame = FrameFacts {
            extra_count: extras.len(),
            bindings: &bindings,
        };
        let facts = |id: &UiComponentId| {
            super::ui_elements::fits_area(id, area).unwrap_or_else(|| {
                frame_fact(app, ui, buffer, preview.is_some(), &frame, id.as_str())
            })
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
        let editor = regions
            .iter()
            .find(|region| &region.id == ui.layout.regions.id(UiFrameRole::Editor));
        let actual = editor.is_none_or(|region| region.area.height == 0);
        if actual == collapsed || attempt == 2 {
            let editor_width = editor.map_or(area.width, |region| {
                let rect = region.area;
                metrics
                    .editor
                    .insets
                    .text_width(rect.width, rect.width > 2 && rect.height > 2)
            });
            return FrameSnapshot {
                regions,
                area,
                editor_width,
                extras,
                bindings,
            };
        }
        collapsed = actual;
    }
    unreachable!("bounded frame snapshot loop")
}

struct FrameMeasurements<'a> {
    editor: EditorMeasurements<'a>,
    review_count: u16,
    review_detail_rows: u16,
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
        let review_detail_rows = app.tool_review_prompt.as_ref().map_or(0, |prompt| {
            let mut rows = 0usize;
            for (role, body) in prompt.details.iter() {
                if !role.is_empty() {
                    rows += 1;
                }
                rows += body.lines().count().max(usize::from(!body.is_empty()));
            }
            u16::try_from(rows.clamp(1, 6)).unwrap_or(6)
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
            review_detail_rows,
            command_count: u16::try_from(app.command_candidates.len()).unwrap_or(u16::MAX),
            command_limit: ui.layout.max_command_rows,
            extension_count,
        }
    }

    fn extent(&self, id: &marlin_agent_ui::UiComponentId) -> super::ui_elements::Extent {
        use super::ui_elements::Extent;
        match id.as_str() {
            "review-count" => Extent::Cells(self.review_count),
            "review-detail-rows" => Extent::Cells(self.review_detail_rows),
            "command-count" => Extent::Cells(self.command_count),
            "command-limit" => Extent::Cells(self.command_limit),
            "extension-count" => Extent::Cells(self.extension_count),
            _ => self.editor.extent(id),
        }
    }
}

pub(crate) fn mouse_over_minibuffer(
    regions: &[super::ui_elements::HitRegion],
    hover: &UiComponentId,
    column: u16,
    row: u16,
) -> bool {
    column > 0
        && row > 0
        && regions.iter().any(|region| {
            &region.id == hover
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
    draw_with_snapshot(frame, app, ui, cache, anchor, &snapshot).scroll
}

pub(crate) struct PaintedFrame {
    pub(crate) scroll: usize,
    pub(crate) regions: Vec<super::ui_elements::HitRegion>,
}

pub(crate) fn draw_with_snapshot(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    ui: &UiWorkbench,
    cache: &mut TranscriptViewportCache,
    anchor: &mut ScrollAnchor,
    snapshot: &FrameSnapshot,
) -> PaintedFrame {
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
            } if id == &ui.focus => Some((label.as_str(), content)),
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
                content,
                app.dynamic_buffer_content.get(buffer).map(Vec::as_slice),
            )),
            _ => None,
        }),
        _ => None,
    };
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
    let static_name = static_content.map(|(label, _, _)| header_value(label));
    let panel_title = static_content
        .map(|(label, _, _)| label)
        .or(focused_content.map(|(label, _)| label))
        .map_or_else(String::new, header_value);
    let extra_one = snapshot.extras.first();
    let extra_two = snapshot.extras.get(1);
    let frame_facts = FrameFacts {
        extra_count: snapshot.extras.len(),
        bindings: &snapshot.bindings,
    };
    let facts = |id: &UiComponentId| {
        super::ui_elements::fits_area(id, area).unwrap_or_else(|| {
            frame_fact(
                app,
                ui,
                buffer,
                preview.is_some(),
                &frame_facts,
                id.as_str(),
            )
        })
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
    let [key_echo, notice] = echo_bindings(app);
    let host_bindings = [
        ("panel-title", panel_title.as_str()),
        ("selected-count", selected_count.as_str()),
        ("candidate-count", candidate_count.as_str()),
        ("prompt", command_prompt.trim_start()),
        ("compact-prompt", command_prompt),
        (
            "query",
            app.history_search
                .as_ref()
                .map_or("", |search| search.query()),
        ),
        ("model", model.as_str()),
        ("session", session.as_str()),
        ("turns", turns.as_str()),
        ("extra-one", extra_one.map_or("", String::as_str)),
        ("extra-two", extra_two.map_or("", String::as_str)),
        key_echo,
        notice,
        ("static-name", static_name.as_deref().unwrap_or("")),
    ];
    let mut bindings = Vec::new();
    let bindings = if snapshot.bindings.is_empty() {
        host_bindings.as_slice()
    } else {
        bindings.extend(
            snapshot
                .bindings
                .iter()
                .map(|(id, value)| (id.as_str(), value.as_str())),
        );
        bindings.extend(host_bindings);
        bindings.as_slice()
    };
    let inputs = UiRenderInputs {
        theme: &ui.theme,
        offset: &|_| 0,
        facts: &facts,
        extent: &|id| extents.extent(id),
        bindings,
    };
    let mut slots = main_view::FrameSlots {
        main: main_view::MainSlots {
            app,
            cache,
            anchor,
            static_content: static_content.map(|(_, content, _)| content),
            focused_content: focused_content.map(|(_, content)| content),
            dynamic_content: static_content.and_then(|(_, _, dynamic)| dynamic),
            effective_scroll: app.scroll_up,
        },
        editor: main_view::EditorSlots {
            composer,
            expansion_bindings: if !command_active
                && app.history_search.is_none()
                && ui.focus.as_str() == "composer"
                && app.key_mode == marlin_agent_ui::UiKeyMode::Normal
            {
                &ui.keymap
            } else {
                &[]
            },
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
    let mut painted = Vec::with_capacity(snapshot.regions.len());
    super::ui_elements::render_with_slots(
        frame,
        area,
        &ui.layout.frame_tree,
        &mut painted,
        &inputs,
        &mut slots,
    );
    if let Some(helm) = app.helm.as_ref()
        && let Some(window_area) = super::helm::helm_window_area(area)
    {
        let helm_tree = super::helm::project_helm_ui_element(helm, window_area);
        super::ui_elements::render_overlay_element(frame, window_area, &helm_tree, &ui.theme);
    }
    PaintedFrame {
        scroll: slots.main.effective_scroll,
        regions: painted,
    }
}
