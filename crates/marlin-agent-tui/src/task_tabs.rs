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
}
