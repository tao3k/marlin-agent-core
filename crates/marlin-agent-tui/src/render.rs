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

pub(crate) fn frame_regions(
    app: &App,
    ui: &UiWorkbench,
    area: Rect,
) -> Vec<super::ui_elements::HitRegion> {
    use super::ui_elements::{Extent, RegionExtents, layout_regions_with_facts};
    let preview = app.preview_buffer();
    let buffer = preview.as_ref().unwrap_or(&app.buffer_view);
    let facts = |id: &marlin_agent_ui::UiComponentId| {
        main_view_fact(app, ui, buffer, preview.is_some(), id.as_str())
    };
    let editor_metrics = editor_measurements(app, &ui.layout, area.width, area.height, &facts);
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
    let metrics = |id: &marlin_agent_ui::UiComponentId| match id.as_str() {
        "review-count" => Extent::Cells(review_count),
        "command-count" => {
            Extent::Cells(u16::try_from(app.command_candidates.len()).unwrap_or(u16::MAX))
        }
        "command-limit" => Extent::Cells(ui.layout.max_command_rows),
        "extension-count" => Extent::Cells(extension_count),
        _ => editor_metrics.extent(id),
    };
    let extents = RegionExtents::new(&ui.layout.frame_tree, &metrics, &facts);
    let mut result = Vec::with_capacity(7);
    layout_regions_with_facts(
        &ui.layout.frame_tree,
        area,
        &|id| extents.extent(id),
        &mut result,
        &facts,
    );
    result
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

pub(crate) fn draw(
    frame: &mut ratatui::Frame<'_>,
    app: &App,
    ui: &UiWorkbench,
    cache: &mut TranscriptViewportCache,
    anchor: &mut ScrollAnchor,
) -> usize {
    app.list_state.borrow_mut().begin_frame();
    let area = frame.area();
    frame.buffer_mut().set_style(
        area,
        Style::default()
            .bg(rgb(ui.theme.canvas))
            .fg(rgb(ui.theme.ink)),
    );
    // The transcript can end in a separator row, so reserve two rows when
    // possible. Bound measurement for large drafts.
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
            } => Some((id, label)),
            _ => None,
        })
        .collect();
    let status_items = ui.regions.iter().find_map(|region| match region {
        UiRegion::Status(items) if !items.is_empty() => Some(items),
        _ => None,
    });
    let suggestions = if app.history_search.is_none()
        && app.command_draft.is_some()
        && app.tool_review_prompt.is_none()
    {
        app.command_candidates.as_slice()
    } else {
        &[]
    };
    let regions = frame_regions(app, ui, area);
    let part = |name: &str| {
        regions
            .iter()
            .find(|region| region.id.as_str() == name)
            .map_or(Rect::default(), |region| region.area)
    };
    let parts = [
        part("transcript"),
        part("extensions"),
        part("modeline"),
        part("editor"),
        part("candidates"),
        part("minibuffer"),
    ];
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
    let main_facts = |id: &marlin_agent_ui::UiComponentId| {
        main_view_fact(app, ui, buffer, preview.is_some(), id.as_str())
    };
    let mut slots = main_view::MainSlots {
        app,
        cache,
        anchor,
        role_style,
        static_content,
        focused_content,
        effective_scroll: app.scroll_up,
    };
    if let Some(main_view) = ui.layout.frame_tree.region_child("transcript") {
        let panel_title = static_content
            .or(focused_content)
            .map_or_else(String::new, |(title, _)| header_value(title));
        let main_bindings = [("panel-title", panel_title.as_str())];
        let inputs = super::ui_elements::UiRenderInputs {
            theme: &ui.theme,
            offset: &|_| 0,
            facts: &main_facts,
            extent: &|_| super::ui_elements::Extent::Cells(0),
            bindings: &main_bindings,
        };
        super::ui_elements::render_with_slots(
            frame,
            parts[0],
            main_view,
            &mut Vec::new(),
            &inputs,
            &mut slots,
        );
    }
    let effective_scroll = slots.effective_scroll;
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
    if let Some(editor) = ui.layout.frame_tree.region_child("editor") {
        let facts = |id: &marlin_agent_ui::UiComponentId| {
            main_view_fact(app, ui, buffer, preview.is_some(), id.as_str())
        };
        let bindings = [
            ("selected-count", selected_count.as_str()),
            ("candidate-count", candidate_count.as_str()),
            ("prompt", command_prompt.trim_start()),
            ("compact-prompt", command_prompt),
            ("title", title),
        ];
        let inputs = super::ui_elements::UiRenderInputs {
            theme: &ui.theme,
            offset: &|_| 0,
            facts: &facts,
            extent: &|_| super::ui_elements::Extent::Cells(0),
            bindings: &bindings,
        };
        let mut editor_slots = main_view::EditorSlots {
            composer,
            show_cursor: command_active
                || (app.history_search.is_none() && ui.focus.as_str() == "composer"),
            read_only: app.tool_review_prompt.is_some(),
        };
        super::ui_elements::render_with_slots(
            frame,
            parts[3],
            editor,
            &mut Vec::new(),
            &inputs,
            &mut editor_slots,
        );
    }
    let menu_prefix = app.command_menu.as_ref().map(|menu| menu.prefix.as_str());
    let name_width = suggestions
        .iter()
        .map(|name| UnicodeWidthStr::width(menu_display_name(name.as_str(), menu_prefix)))
        .max()
        .unwrap_or(0);
    if let Some(template) = ui.layout.frame_tree.region_child("candidates") {
        let count = app
            .tool_review_prompt
            .as_ref()
            .map_or(suggestions.len(), |prompt| prompt.menu.choices.len());
        super::ui_elements::render_list(
            frame,
            parts[4],
            template,
            &ui.theme,
            super::ui_elements::UiListData {
                count,
                selected: selected_index,
                extent: &|id| {
                    if id.as_str() == "candidate-name-width" {
                        super::ui_elements::Extent::Cells(
                            u16::try_from(name_width).unwrap_or(u16::MAX),
                        )
                    } else {
                        super::ui_elements::Extent::Cells(0)
                    }
                },
            },
            &mut app.list_state.borrow_mut(),
            |index| {
                let (key, bindings) = if let Some(prompt) = &app.tool_review_prompt {
                    let choice = prompt.menu.choices.get(index)?;
                    (
                        None,
                        vec![
                            ("name", choice.label.as_str().into()),
                            ("description", choice.description.as_str().into()),
                        ],
                    )
                } else {
                    let name = suggestions.get(index)?;
                    let command = ui.commands().iter().find(|command| &command.name == name)?;
                    (
                        Some(marlin_agent_ui::UiComponentId::new(name.as_str()).ok()?),
                        vec![
                            (
                                "name",
                                menu_display_name(command.name.as_str(), menu_prefix).into(),
                            ),
                            ("description", header_value(&command.description).into()),
                        ],
                    )
                };
                Some(super::ui_elements::UiListRow { key, bindings })
            },
        );
    }
    if let Some(template) = ui.layout.frame_tree.region_child("extensions") {
        let focused = extensions.iter().position(|(id, _)| *id == &ui.focus);
        super::ui_elements::render_list(
            frame,
            parts[1],
            template,
            &ui.theme,
            super::ui_elements::UiListData {
                count: extensions.len(),
                selected: focused,
                extent: &|_| super::ui_elements::Extent::Cells(0),
            },
            &mut app.list_state.borrow_mut(),
            |index| {
                Some(super::ui_elements::UiListRow {
                    key: None,
                    bindings: vec![("label", header_value(extensions.get(index)?.1).into())],
                })
            },
        );
    }
    if parts[2].height > 0
        && let Some(bar) = ui.layout.frame_tree.region_child("modeline")
    {
        let activity = match app.modeline_activity() {
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
        };
        let model = header_value(&ui.model_label);
        let session = header_value(&ui.session_label);
        let turns = (app.history.len() / 2).to_string();
        let bindings = [
            ("model", model.as_str()),
            ("session", session.as_str()),
            ("turns", turns.as_str()),
        ];
        let inputs = super::ui_elements::UiRenderInputs {
            theme: &ui.theme,
            offset: &|_| 0,
            facts: &|id| {
                id.as_str() == activity || id.as_str() == "has-turns" && app.history.len() >= 2
            },
            extent: &|_| super::ui_elements::Extent::Cells(0),
            bindings: &bindings,
        };
        super::ui_elements::render_with_facts(frame, parts[2], bar, &mut Vec::new(), &inputs);
    }
    if parts[5].height == 0 {
        return effective_scroll;
    }
    let static_name = static_content.map(|(label, _)| header_value(label));
    if app.history_search.is_none()
        && let Some(hint) = &app.transient_hint
    {
        super::ui_elements::render_element(frame, parts[5], hint, &ui.theme);
        return effective_scroll;
    }
    let mut extras = status_items
        .into_iter()
        .flatten()
        .filter(|_| parts[3].height == 0)
        .filter(|item| !item.id.as_str().starts_with("modeline/"))
        .take(2)
        .map(|item| display_value(&item.label, 16));
    let extra_one = extras.next();
    let extra_two = extras.next();
    if let Some(view) = ui.layout.frame_tree.region_child("minibuffer") {
        let facts = |fact: &marlin_agent_ui::UiComponentId| match fact.as_str() {
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
            "messages" => *buffer == BufferView::Messages,
            "static-buffer" => static_name.is_some(),
            "editor-collapsed" => parts[3].height == 0,
            "extra-one" => extra_one.is_some(),
            "extra-two" => extra_two.is_some(),
            "quit-armed" => app.quit_armed,
            "draft-limit" => app.composer.limit_reached(),
            "runtime" => preview.is_none() && app.runtime_receipt_open,
            "workspace" => preview.is_none() && app.workspace.view().is_some(),
            "commands-focus" => ui.focus.as_str() == "commands",
            "transcript-focus" => ui.focus.as_str() == "transcript",
            "extension-focus" => ui.focus.as_str() != "composer",
            "normal-mode" => !command_active && app.key_mode == marlin_agent_ui::UiKeyMode::Normal,
            _ => false,
        };
        let bindings = [
            ("extra-one", extra_one.as_deref().unwrap_or("")),
            ("extra-two", extra_two.as_deref().unwrap_or("")),
            (
                "notice",
                app.minibuffer_notice.as_deref().unwrap_or("").trim_start(),
            ),
            ("static-name", static_name.as_deref().unwrap_or("")),
        ];
        let extent = |id: &marlin_agent_ui::UiComponentId| {
            let value = match id.as_str() {
                "extra-one-width" => extra_one.as_deref(),
                "extra-two-width" => extra_two.as_deref(),
                _ => None,
            };
            super::ui_elements::Extent::Cells(value.map_or(0, |text| {
                u16::try_from(UnicodeWidthStr::width(text)).unwrap_or(u16::MAX)
            }))
        };
        let inputs = super::ui_elements::UiRenderInputs {
            theme: &ui.theme,
            offset: &|_| 0,
            facts: &facts,
            extent: &extent,
            bindings: &bindings,
        };
        if let Some(search) = &app.history_search {
            if let Some(cursor) = super::ui_elements::render_prompt(
                frame,
                parts[5],
                view,
                &inputs,
                "query",
                search.query(),
            ) {
                frame.set_cursor_position(cursor);
            }
            return effective_scroll;
        }
        super::ui_elements::render_with_facts(frame, parts[5], view, &mut Vec::new(), &inputs);
    }
    if parts[3].height == 0
        && ui.focus.as_str() == "composer"
        && parts[5].width > 2
        && app.transient_hint.is_none()
    {
        frame.set_cursor_position((parts[5].x + 2, parts[5].y));
    }
    effective_scroll
}
