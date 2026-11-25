// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Pure Ratatui Workbench tab strip renderer for Session-free `GraphEngine` `TaskActor`s.
//!
//! Renders `marlin_agent_runtime::WorkbenchTabProjection` with slot 0 pinned to the
//! `Tasks` Index, active writable task tabs with monotonic fencing epoch badges, and
//! strictly disjoint read-only peek tabs.

use marlin_agent_runtime::{WorkbenchTabEntry, WorkbenchTabKind, WorkbenchTabProjection};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

fn status_glyph(entry: &WorkbenchTabEntry) -> &'static str {
    let lower = entry.status_label.to_ascii_lowercase();
    if lower.contains("steer") {
        "⚑"
    } else if lower.contains("run") {
        "●"
    } else if lower.contains("open") || lower.contains("focus") {
        "○"
    } else {
        "·"
    }
}

/// Projects a canonical `WorkbenchTabProjection` into a styled single-line Ratatui tab strip.
pub fn workbench_tab_strip_line(projection: &WorkbenchTabProjection) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(projection.tabs.len() * 2);
    let index_focused = projection.focused_task_id.is_none();

    for (index, entry) in projection.tabs.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        }
        match entry.kind {
            WorkbenchTabKind::PinnedIndex => {
                let style = if index_focused {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                };
                spans.push(Span::styled(format!(" ★ {} ", entry.title), style));
            }
            WorkbenchTabKind::ActiveTask => {
                let glyph = status_glyph(entry);
                let label = format!(" {glyph} {} · e{} ", entry.title, entry.fencing_epoch);
                let style = if entry.focused {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                spans.push(Span::styled(label, style));
            }
            WorkbenchTabKind::ReadOnlyPeek => {
                let label = format!(" 👁 {} (RO) ", entry.title);
                let style = Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC);
                spans.push(Span::styled(label, style));
            }
        }
    }

    Line::from(spans)
}

/// Renders the Session-free `TaskActor` Workbench tab strip into a 1-row Ratatui region.
pub fn render_workbench_tab_strip(
    frame: &mut Frame<'_>,
    area: Rect,
    projection: &WorkbenchTabProjection,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let bar_area = Rect::new(area.x, area.y, area.width, 1);
    let line = workbench_tab_strip_line(projection);
    frame.render_widget(Paragraph::new(line), bar_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use marlin_agent_runtime::{
        DispatchRequestId, ExecutionReceiptStatus, FencingEpoch, GraphConvergenceNodeId,
        GraphDispatchOperation, GraphSubstrateTarget, MarlinDispatchEnvelope,
        MarlinExecutionReceipt, TaskActorId, TaskActorRuntimeRegistry, TaskActorScopeMode,
    };
    use ratatui::{Terminal, backend::TestBackend};

    fn render_row(projection: &WorkbenchTabProjection, width: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, 1)).unwrap();
        terminal
            .draw(|frame| render_workbench_tab_strip(frame, frame.area(), projection))
            .unwrap();
        let buffer = terminal.backend().buffer();
        (0..width)
            .map(|x| buffer[(x, 0)].symbol())
            .collect::<String>()
    }

    #[test]
    fn renders_pinned_index_active_steering_and_disjoint_read_only_peek() {
        let registry = TaskActorRuntimeRegistry::new();
        let alpha = TaskActorId::new("task-alpha").unwrap();
        let beta = TaskActorId::new("task-beta").unwrap();
        let archived = TaskActorId::new("task-old-7").unwrap();

        registry
            .register_actor(alpha.clone(), "Auth", TaskActorScopeMode::Project)
            .unwrap();
        registry
            .register_actor(beta.clone(), "Graph", TaskActorScopeMode::Project)
            .unwrap();

        // Advance beta into WaitingSteering via C4 ApplySteering.
        registry
            .admit_dispatch(MarlinDispatchEnvelope {
                request_id: DispatchRequestId::new("req-steer-ui").unwrap(),
                task_id: beta.clone(),
                operation: GraphDispatchOperation::ApplySteering,
                fencing_epoch: FencingEpoch::new(1).unwrap(),
                scope_mode: TaskActorScopeMode::Project,
                join_node_id: Some(GraphConvergenceNodeId::new("join-ui").unwrap()),
                substrate_target: GraphSubstrateTarget::PureMemory,
            })
            .unwrap();
        registry
            .reconcile_receipt(&MarlinExecutionReceipt {
                request_id: DispatchRequestId::new("req-steer-ui").unwrap(),
                task_id: beta.clone(),
                fencing_epoch: FencingEpoch::new(1).unwrap(),
                status: ExecutionReceiptStatus::Ok,
            })
            .unwrap();

        registry.set_focus(Some(beta)).unwrap();
        let projection = registry.open_read_only_peek(archived).unwrap();

        let rendered = render_row(&projection, 100);
        let idx_pos = rendered.find("★ Tasks").expect("pinned index at slot 0");
        let alpha_pos = rendered.find("Auth · e1").expect("active alpha tab");
        let beta_pos = rendered.find("⚑ Graph · e2").expect("steering beta tab");
        let peek_pos = rendered
            .find("👁 task-old-7 (RO)")
            .expect("disjoint read-only peek tab");

        assert!(idx_pos < alpha_pos);
        assert!(alpha_pos < beta_pos);
        assert!(beta_pos < peek_pos);
    }

    #[test]
    fn app_main_frame_mounts_task_tab_strip_only_when_actors_or_peeks_exist() {
        use crate::{
            app::App,
            render::{draw, frame_snapshot},
            ui_fixture::{test_frame, test_layout},
            viewport::{ScrollAnchor, TranscriptViewportCache},
        };
        use marlin_agent_protocol::ModelEndpoint;
        use marlin_agent_ui::{UiComponentId, UiRgb, UiTheme, UiWorkbench};

        let ui = UiWorkbench {
            layout: test_layout(),
            theme: UiTheme {
                canvas: UiRgb {
                    red: 0,
                    green: 0,
                    blue: 0,
                },
                bar: UiRgb {
                    red: 20,
                    green: 20,
                    blue: 20,
                },
                ink: UiRgb {
                    red: 240,
                    green: 240,
                    blue: 240,
                },
                muted: UiRgb {
                    red: 120,
                    green: 120,
                    blue: 120,
                },
                accent: UiRgb {
                    red: 80,
                    green: 180,
                    blue: 255,
                },
                info: UiRgb {
                    red: 100,
                    green: 180,
                    blue: 255,
                },
                success: UiRgb {
                    red: 80,
                    green: 200,
                    blue: 120,
                },
                warning: UiRgb {
                    red: 240,
                    green: 180,
                    blue: 60,
                },
            },
            frame: test_frame(),
            regions: Vec::new(),
            keymap: Vec::new(),
            transients: Vec::new(),
            focus: UiComponentId::new("composer").unwrap(),
            model_label: "model".to_owned(),
            session_label: "session".to_owned(),
        };

        let app = App::new(ModelEndpoint::new("provider", "model"));
        let full = Rect::new(0, 0, 80, 12);

        // Baseline: zero tasks -> full height preserved untouched.
        assert!(app.task_actor_tabs().is_none());
        let baseline = frame_snapshot(&app, &ui, full);
        assert_eq!(baseline.area, full);

        // Register an active task -> row 0 hosts the pinned-Index tab strip, body shifts to y=1.
        let task_id = TaskActorId::new("task-live-1").unwrap();
        app.task_actors
            .register_actor(task_id.clone(), "Live Graph", TaskActorScopeMode::Project)
            .unwrap();
        app.task_actors.set_focus(Some(task_id)).unwrap();
        assert!(app.task_actor_tabs().is_some());

        let with_tabs = frame_snapshot(&app, &ui, full);
        assert_eq!(with_tabs.area, Rect::new(0, 1, 80, 11));

        let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
        let mut cache = TranscriptViewportCache::default();
        let mut anchor = ScrollAnchor::default();
        terminal
            .draw(|frame| {
                draw(frame, &app, &ui, &mut cache, &mut anchor);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let top_row: String = (0..80).map(|x| buffer[(x, 0)].symbol()).collect();
        assert!(top_row.contains("★ Tasks"));
        assert!(top_row.contains("Live Graph · e1"));
    }

    #[test]
    fn task_tab_commands_cycle_focus_and_manage_disjoint_read_only_peeks() {
        use crate::{
            app::App,
            command::{Command, TaskTabCommand},
        };
        use marlin_agent_protocol::ModelEndpoint;
        use marlin_agent_ui::{
            UiCommand, UiCommandArguments, UiCommandCapability, UiCommandName,
        };

        let descriptor = |name: &str, args: UiCommandArguments, value: &str| UiCommand {
            name: UiCommandName::new(name).unwrap(),
            description: "task tab action".to_owned(),
            arguments: args,
            capability: UiCommandCapability::TaskTabs,
            value: Some(value.to_owned()),
        };

        let next_desc = descriptor("task-next", UiCommandArguments::None, "next");
        let prev_desc = descriptor("task-prev", UiCommandArguments::None, "prev");
        let idx_desc = descriptor("task-index", UiCommandArguments::None, "index");
        let focus_desc = descriptor("task-focus", UiCommandArguments::Required, "focus");
        let peek_desc = descriptor("task-peek", UiCommandArguments::Required, "peek");
        let unpeek_desc = descriptor("task-unpeek", UiCommandArguments::Required, "unpeek");
        let bad_desc = descriptor("task-bad", UiCommandArguments::None, "peek");

        assert!(Command::admit(&next_desc));
        assert!(Command::admit(&focus_desc));
        assert!(!Command::admit(&bad_desc));
        assert_eq!(
            Command::parse(&next_desc, "task-next"),
            Some(Command::TaskTab(TaskTabCommand::Next))
        );
        assert_eq!(
            Command::parse(&prev_desc, "task-prev"),
            Some(Command::TaskTab(TaskTabCommand::Prev))
        );
        assert_eq!(
            Command::parse(&idx_desc, "task-index"),
            Some(Command::TaskTab(TaskTabCommand::FocusIndex))
        );
        assert_eq!(
            Command::parse(&focus_desc, "task-focus task-beta"),
            Some(Command::TaskTab(TaskTabCommand::Focus("task-beta")))
        );
        assert_eq!(
            Command::parse(&peek_desc, "task-peek task-old"),
            Some(Command::TaskTab(TaskTabCommand::Peek("task-old")))
        );
        assert_eq!(
            Command::parse(&unpeek_desc, "task-unpeek task-old"),
            Some(Command::TaskTab(TaskTabCommand::Unpeek("task-old")))
        );

        let mut app = App::new(ModelEndpoint::new("provider", "model"));
        let alpha = TaskActorId::new("task-alpha").unwrap();
        let beta = TaskActorId::new("task-beta").unwrap();
        app.task_actors
            .register_actor(alpha.clone(), "Alpha", TaskActorScopeMode::Project)
            .unwrap();
        app.task_actors
            .register_actor(beta.clone(), "Beta", TaskActorScopeMode::Project)
            .unwrap();

        // Cycle forward: Index -> alpha -> beta -> Index
        app.apply_task_tab_command(TaskTabCommand::Next);
        assert_eq!(
            app.task_actors.project_workbench_tabs().focused_task_id,
            Some(alpha.clone())
        );
        app.apply_task_tab_command(TaskTabCommand::Next);
        assert_eq!(
            app.task_actors.project_workbench_tabs().focused_task_id,
            Some(beta.clone())
        );
        app.apply_task_tab_command(TaskTabCommand::Next);
        assert_eq!(
            app.task_actors.project_workbench_tabs().focused_task_id,
            None
        );

        // Cycle backward from Index wraps to beta
        app.apply_task_tab_command(TaskTabCommand::Prev);
        assert_eq!(
            app.task_actors.project_workbench_tabs().focused_task_id,
            Some(beta)
        );
        app.apply_task_tab_command(TaskTabCommand::FocusIndex);
        assert_eq!(
            app.task_actors.project_workbench_tabs().focused_task_id,
            None
        );

        // Disjoint read-only peek opens and closes cleanly; peeking an active task is denied.
        app.apply_task_tab_command(TaskTabCommand::Peek("task-alpha"));
        assert!(
            !app.task_actors
                .project_workbench_tabs()
                .tabs
                .iter()
                .any(|t| t.kind == WorkbenchTabKind::ReadOnlyPeek)
        );
        app.apply_task_tab_command(TaskTabCommand::Peek("task-archived-42"));
        assert!(
            app.task_actors
                .project_workbench_tabs()
                .tabs
                .iter()
                .any(|t| t.kind == WorkbenchTabKind::ReadOnlyPeek && t.id == "task-archived-42")
        );
        app.apply_task_tab_command(TaskTabCommand::Unpeek("task-archived-42"));
        assert!(
            !app.task_actors
                .project_workbench_tabs()
                .tabs
                .iter()
                .any(|t| t.kind == WorkbenchTabKind::ReadOnlyPeek)
        );
    }
}
