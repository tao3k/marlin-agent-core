// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::TestUiHost;
use marlin_agent_ui::UiOpenFacts;

#[test]
fn linked_scheme_workbench_renders_through_the_native_boundary() {
    let runtime = super::initialize().unwrap();
    let facts = UiOpenFacts {
        model_label: "model",
        session_label: "session",
        status: &[],
        extensions: &[],
        method_bundles: &[],
    };
    // SAFETY: the runtime and exports originate in the same linked programme.
    let host = unsafe { TestUiHost::open_linked(&runtime, super::exports(), &facts) }.unwrap();
    // Scheme tests own the workbench tree. This integration check exercises
    // the linked projection and its effect on native terminal geometry.
    let mut app = super::super::app::App::new(marlin_agent_protocol::ModelEndpoint::new(
        "provider", "model",
    ));
    app.toggle_command();
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
    assert_eq!(editor.height, 1);
    assert!(candidates.height > 0);
    assert_eq!(editor.bottom(), candidates.top());
    assert_eq!(region("modeline").bottom(), editor.top());
}
