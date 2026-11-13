// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::TestUiHost;
use marlin_agent_ui::UiOpenFacts;

#[test]
fn linked_scheme_workbench_renders_through_the_native_boundary() {
    let runtime = super::initialize().unwrap();
    let statuses = [
        marlin_agent_ui::UiStatusItem::host(
            marlin_agent_ui::UiComponentId::new("extra/one").unwrap(),
            "鱼",
        ),
        marlin_agent_ui::UiStatusItem::host(
            marlin_agent_ui::UiComponentId::new("extra/two").unwrap(),
            "ready",
        ),
    ];
    let facts = UiOpenFacts {
        model_label: "model",
        session_label: "session",
        status: &statuses,
        extensions: &[],
        method_bundles: &[],
    };
    // SAFETY: the runtime and exports originate in the same linked programme.
    let mut host = unsafe { TestUiHost::open_linked(&runtime, super::exports(), &facts) }.unwrap();
    let candidates = host.command_candidates("mode", None).unwrap().names;
    assert!(!candidates.is_empty());
    assert_eq!(
        host.browse_command_steps(
            "mode",
            &candidates,
            None,
            marlin_agent_ui::UiBrowseDirection::Next,
            33,
            true,
        )
        .unwrap(),
        Some(candidates[32 % candidates.len()].clone())
    );
    // Scheme tests own the workbench tree. This integration check exercises
    // the linked projection and its effect on native terminal geometry.
    let mut app = super::super::app::App::new(marlin_agent_protocol::ModelEndpoint::new(
        "provider", "model",
    ));
    let mut status_terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 12)).unwrap();
    let mut cache = super::super::viewport::TranscriptViewportCache::default();
    let mut anchor = super::super::viewport::ScrollAnchor::default();
    status_terminal
        .draw(|frame| {
            super::super::render::draw(frame, &app, host.view(), &mut cache, &mut anchor);
        })
        .unwrap();
    let row = status_terminal.backend().buffer();
    assert_eq!(row[(3, 11)].symbol(), "鱼");
    assert_eq!(row[(6, 11)].symbol(), "·");
    assert_eq!(row[(8, 11)].symbol(), "r");
    assert_eq!(row[(16, 11)].symbol(), "M");
    app.toggle_command();
    app.command_candidates = vec![marlin_agent_ui::UiCommandName::new("help").unwrap()];
    let regions = super::super::render::frame_regions(
        &app,
        host.view(),
        ratatui::layout::Rect::new(0, 0, 80, 12),
    );
    let region = |name| {
        regions
            .iter()
            .find(|region| region.id.as_str() == name)
            .unwrap()
            .area
    };
    let editor = region("editor");
    let candidates = region("candidates");
    let step = |name| {
        regions
            .iter()
            .find(|region| region.id.as_str() == name)
            .and_then(|region| region.scroll_step())
    };
    assert_eq!(step("transcript"), Some(3));
    assert_eq!(step("candidates"), Some(1));
    assert_eq!(editor.height, 1);
    assert_eq!(candidates.height, 1);
    assert_eq!(editor.bottom(), candidates.top());
    assert_eq!(region("modeline").bottom(), editor.top());

    // Candidate count crosses the real Scheme-to-Ratatui measurement boundary.
    for (count, expected) in [(0, 0), (3, 3), (20, 10)] {
        app.command_candidates = (0..count)
            .map(|index| marlin_agent_ui::UiCommandName::new(&format!("command-{index}")).unwrap())
            .collect();
        let regions = super::super::render::frame_regions(
            &app,
            host.view(),
            ratatui::layout::Rect::new(0, 0, 80, 40),
        );
        let height = regions
            .iter()
            .find(|region| region.id.as_str() == "candidates")
            .map_or(0, |region| region.area.height);
        assert_eq!(height, expected);
    }

    // Evaluate the linked Scheme list policy across consecutive selections.
    let list = host
        .view()
        .layout
        .frame_tree
        .region_child("candidates")
        .unwrap();
    let mut state = super::super::ui_elements::UiListState::default();
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 5)).unwrap();
    for (selected, expected_start) in [(0, 0), (9, 5), (10, 6), (5, 5)] {
        let mut rows = Vec::new();
        terminal
            .draw(|frame| {
                super::super::ui_elements::render_list(
                    frame,
                    frame.area(),
                    list,
                    &host.view().theme,
                    super::super::ui_elements::UiListData {
                        count: 20,
                        selected: Some(selected),
                        extent: &|_| super::super::ui_elements::Extent::Cells(0),
                        facts: &|_| false,
                        bindings: &[],
                    },
                    &mut state,
                    |index| {
                        rows.push(index);
                        Some(super::super::ui_elements::UiListRow {
                            key: None,
                            bindings: vec![],
                        })
                    },
                );
            })
            .unwrap();
        assert_eq!(
            rows,
            (expected_start..expected_start + 5).collect::<Vec<_>>()
        );
    }
    host.refresh_labels("provider/next", "next-session")
        .unwrap();
    assert_eq!(host.view().model_label, "provider/next");
    assert_eq!(host.view().session_label, "next-session");
}
