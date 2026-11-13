// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_ui::{UiComponentId, UiEvent, UiIntent, UiLayoutPolicy, UiRegion, UiWorkbench};

fn test_layout() -> UiLayoutPolicy {
    UiLayoutPolicy {
        message_frame: marlin_agent_ui::UiFrameSpec {
            min_transcript_rows: 2,
            fixed_editor_rows: 0,
            footer_rows: 1,
            extensions: true,
            bordered: true,
            prompt: " Message ".into(),
        },
        command_frame: marlin_agent_ui::UiFrameSpec {
            min_transcript_rows: 0,
            fixed_editor_rows: 1,
            footer_rows: 0,
            extensions: false,
            bordered: false,
            prompt: " M-x ".into(),
        },
        max_chatbox_rows: 10,
        max_command_rows: 32,
        frame_tree: marlin_agent_ui::UiElement::Rows(
            [
                "transcript",
                "extensions",
                "modeline",
                "editor",
                "candidates",
                "minibuffer",
            ]
            .into_iter()
            .map(|name| marlin_agent_ui::UiElement::Region {
                id: UiComponentId::new(name).unwrap(),
                scrollable: matches!(name, "transcript" | "candidates"),
                child: Box::new(marlin_agent_ui::UiElement::Text(vec![])),
            })
            .collect(),
        ),
        transcript_policy: marlin_agent_ui::UiTranscriptPolicy {
            default_collapsed: true,
            user_prefix: "u: ".into(),
            assistant_role: String::new(),
            thinking_collapsed_role: "closed".into(),
            thinking_expanded_role: "open".into(),
        },
    }
}

fn test_frame() -> marlin_agent_ui::UiFrame {
    use marlin_agent_ui::{UiBufferId, UiFrame, UiWindow};
    let id = |name| UiComponentId::new(name).unwrap();
    let buffer = |name| UiBufferId::new(name).unwrap();
    UiFrame {
        id: id("main"),
        windows: vec![UiWindow {
            id: id("main"),
            buffer: buffer("conversation"),
        }],
        selected_window: id("main"),
        minibuffer: id("minibuffer"),
        buffers: vec![buffer("conversation"), buffer("messages")],
    }
}

fn id(value: &str) -> UiComponentId {
    UiComponentId::new(value).unwrap()
}

fn workbench(focus: &str) -> UiWorkbench {
    UiWorkbench {
        layout: test_layout(),
        theme: marlin_agent_ui::UiTheme::default(),
        model_label: "provider/model".into(),
        session_label: "session".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Extension {
                id: id("guide"),
                label: "Guide".into(),
                focusable: true,
                content: vec![],
            },
        ],
        focus: id(focus),
        frame: test_frame(),
        keymap: Vec::new(),
        transients: Vec::new(),
    }
}

#[test]
fn shared_ui_admission_rejects_effect_and_snapshot_substitution() {
    let composer = workbench("composer");
    let guide = workbench("guide");
    let focus = UiIntent::Focus(id("guide"));
    assert!(
        composer
            .admit_successor(&guide, UiEvent::NextFocus, &focus)
            .is_ok()
    );
    assert!(
        composer
            .admit_successor(&guide, UiEvent::Submit, &focus)
            .is_err()
    );

    let mut altered = guide.clone();
    altered.regions[2] = UiRegion::Transcript;
    assert!(
        composer
            .admit_successor(&altered, UiEvent::NextFocus, &focus)
            .is_err()
    );
    let mut changed_layout = guide.clone();
    changed_layout.layout.max_chatbox_rows = 5;
    assert!(
        composer
            .admit_successor(&changed_layout, UiEvent::NextFocus, &focus)
            .is_err()
    );
    assert!(
        guide
            .admit_successor(&composer, UiEvent::Submit, &UiIntent::Focus(id("composer")))
            .is_ok()
    );
}

#[test]
fn profile_admission_preserves_status_ownership_while_allowing_modeline_order() {
    use marlin_agent_ui::{UiProfileName, UiStatusItem, UiStatusTone};
    let modeline = |identity: &str, label: &str, keep_priority| UiStatusItem {
        id: id(identity),
        label: label.into(),
        compact_label: label.into(),
        tone: UiStatusTone::Info,
        active_tone: UiStatusTone::Info,
        min_width: 0,
        compact_below: 0,
        keep_priority,
    };
    let mut previous = workbench("composer");
    previous.regions.push(UiRegion::Status(vec![
        modeline("modeline/left/brand", "brand", 0),
        modeline("modeline/left/model", "  {value}  ", 3),
        modeline("modeline/left/activity", "ready", 0),
        modeline("modeline/right/turns", "T{value}  ", 1),
        modeline("modeline/right/session", " § {value} ", 2),
        modeline("modeline/state/online", "online", 0),
        modeline("modeline/state/offline", "offline", 0),
        modeline("modeline/state/unknown", "unknown", 0),
        modeline("modeline/state/runtime", "runtime", 0),
        modeline("modeline/state/workspace", "workspace", 0),
        modeline("modeline/state/recovery", "recovery", 0),
        modeline("modeline/state/continuation", "continuation", 0),
        modeline("modeline/state/history", "history", 0),
        modeline("modeline/state/working", "working", 0),
        modeline("modeline/state/reading", "reading", 0),
        UiStatusItem::host(id("host/status"), "host fact"),
    ]));
    let event = UiEvent::SelectProfile(UiProfileName::new("pi").unwrap());
    let focus = UiIntent::Focus(id("composer"));
    let mut reordered = previous.clone();
    reordered.layout.transcript_policy.thinking_collapsed_role = "⋯".into();
    if let UiRegion::Status(items) = &mut reordered.regions[3] {
        items.swap(3, 4);
        items[0].label = "pi brand".into();
        items[1].keep_priority = 1;
        items[3].keep_priority = 3;
    }
    assert!(
        previous
            .admit_successor(&reordered, event.clone(), &focus)
            .is_ok()
    );

    let mut changed_geometry = reordered.clone();
    changed_geometry.layout.max_command_rows += 1;
    assert!(
        previous
            .admit_successor(&changed_geometry, event.clone(), &focus)
            .is_err()
    );

    let mut changed_host = reordered.clone();
    if let UiRegion::Status(items) = &mut changed_host.regions[3] {
        items[15].label = "forged".into();
    }
    assert!(
        previous
            .admit_successor(&changed_host, event.clone(), &focus)
            .is_err()
    );
    let mut duplicate_role = reordered.clone();
    if let UiRegion::Status(items) = &mut duplicate_role.regions[3] {
        items[8].id = items[7].id.clone();
    }
    assert!(
        previous
            .admit_successor(&duplicate_role, event.clone(), &focus)
            .is_err()
    );
    let mut missing_role = reordered;
    if let UiRegion::Status(items) = &mut missing_role.regions[3] {
        items[0].id = id("modeline/left/other");
    }
    assert!(
        previous
            .admit_successor(&missing_role, event, &focus)
            .is_err()
    );
}
