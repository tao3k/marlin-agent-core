// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Measures the admitted native path after the Scheme snapshot is projected.
//! VM open, module resolution and first frame are separate startup workloads.

use criterion::{Criterion, criterion_group, criterion_main};
use marlin_agent_ui::{
    UiBufferId, UiCommandName, UiComponentId, UiEditorAction, UiEvent, UiFrame, UiFrameSpec,
    UiIntent, UiKeyBinding, UiKeyCommand, UiKeyToken, UiLayoutPolicy, UiProfileName, UiRegion,
    UiStatusItem, UiStatusTone, UiTheme, UiWindow, UiWorkbench,
};
use std::{hint::black_box, time::Duration};

fn id(value: &str) -> UiComponentId {
    UiComponentId::new(value).unwrap()
}

fn buffer(value: &str) -> UiBufferId {
    UiBufferId::new(value).unwrap()
}

fn modeline(
    identity: &str,
    label: &str,
    compact_label: &str,
    min_width: u16,
    compact_below: u16,
) -> UiStatusItem {
    UiStatusItem {
        id: id(identity),
        label: label.into(),
        compact_label: compact_label.into(),
        tone: UiStatusTone::Info,
        active_tone: UiStatusTone::Info,
        min_width,
        compact_below,
        keep_priority: match identity {
            "modeline/left/model" => 3,
            "modeline/right/session" => 2,
            "modeline/right/turns" => 1,
            _ => 0,
        },
    }
}

fn frame() -> UiFrame {
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

fn view() -> UiWorkbench {
    UiWorkbench {
        model_label: "deepseek/deepseek-v4-pro".into(),
        session_label: "temporary".into(),
        regions: vec![
            UiRegion::Transcript,
            UiRegion::Composer,
            UiRegion::Commands(Vec::new()),
            UiRegion::Status(Vec::new()),
        ],
        layout: UiLayoutPolicy {
            message_frame: UiFrameSpec {
                min_transcript_rows: 2,
                fixed_editor_rows: 0,
                footer_rows: 1,
                extensions: true,
                bordered: true,
                prompt: " Message ".into(),
            },
            command_frame: UiFrameSpec {
                min_transcript_rows: 0,
                fixed_editor_rows: 1,
                footer_rows: 0,
                extensions: false,
                bordered: false,
                prompt: " M-x ".into(),
            },
            max_chatbox_rows: 10,
            max_command_rows: 32,
            transcript_policy: marlin_agent_ui::UiTranscriptPolicy {
                default_collapsed: true,
                user_prefix: "u: ".into(),
                assistant_role: String::new(),
                thinking_collapsed_role: "closed".into(),
                thinking_expanded_role: "open".into(),
            },
        },
        theme: UiTheme::default(),
        focus: id("composer"),
        frame: frame(),
        keymap: [
            (vec!["M-x"], UiKeyCommand::OpenCommands),
            (vec!["C-o"], UiKeyCommand::OpenCommands),
            (vec!["C-g"], UiKeyCommand::Cancel),
            (
                vec!["C-r"],
                UiKeyCommand::Edit(UiEditorAction::HistorySearch),
            ),
            (
                vec!["M-p"],
                UiKeyCommand::Edit(UiEditorAction::PreviousPrompt),
            ),
            (vec!["M-n"], UiKeyCommand::Edit(UiEditorAction::NextPrompt)),
            (
                vec!["C-n"],
                UiKeyCommand::Edit(UiEditorAction::InsertNewline),
            ),
            (vec!["C-a"], UiKeyCommand::Edit(UiEditorAction::LineStart)),
            (vec!["C-e"], UiKeyCommand::Edit(UiEditorAction::LineEnd)),
            (
                vec!["C-x", "b"],
                UiKeyCommand::Invoke(UiCommandName::new("switch-buffer").unwrap()),
            ),
        ]
        .into_iter()
        .map(|(keys, command)| UiKeyBinding {
            sequence: keys
                .into_iter()
                .map(|key| UiKeyToken::new(key).unwrap())
                .collect(),
            command,
        })
        .collect(),
    }
}

fn benchmark_hot_path(criterion: &mut Criterion) {
    let prior = view();
    let mut selected = prior.clone();
    selected.frame.windows[0].buffer = buffer("messages");
    let event = UiEvent::SelectBuffer(buffer("messages"));
    let intent = UiIntent::SelectBuffer(buffer("messages"));
    let mut profile_prior = view();
    profile_prior.regions[3] = UiRegion::Status(vec![
        modeline("modeline/left/brand", "🐟 MARLIN", "🐟", 0, 18),
        modeline("modeline/left/model", "  {value}  ", "{value}", 0, 0),
        modeline("modeline/left/activity", "🟡 Ready", "🟡", 0, 64),
        modeline("modeline/right/turns", "T{value}  ", "T{value}", 72, 0),
        modeline("modeline/right/session", " § {value} ", "§{value}", 44, 0),
        modeline("modeline/state/online", "🟢 Online", "🟢", 0, 64),
        modeline("modeline/state/offline", "🔴 Offline", "🔴", 0, 64),
        modeline("modeline/state/unknown", "🟠 Unknown", "🟠", 0, 64),
        modeline("modeline/state/runtime", "runtime", "r", 0, 64),
        modeline("modeline/state/workspace", "workspace", "w", 0, 64),
        modeline("modeline/state/recovery", "recovery", "r", 0, 64),
        modeline("modeline/state/continuation", "blocked", "b", 0, 64),
        modeline("modeline/state/history", "history", "h", 0, 64),
        modeline("modeline/state/working", "working", "w", 0, 64),
        modeline("modeline/state/reading", "reading", "r", 0, 64),
        UiStatusItem::host(id("host/status"), "host fact"),
    ]);
    let mut profile_next = profile_prior.clone();
    if let UiRegion::Status(items) = &mut profile_next.regions[3] {
        items[0].label = "🐟 marlin".into();
        items[3].min_width = 66;
        items[1].keep_priority = 1;
        items[4].keep_priority = 3;
        items.swap(3, 4);
    }
    let profile_event = UiEvent::SelectProfile(UiProfileName::new("pi").unwrap());
    let profile_intent = UiIntent::Focus(id("composer"));
    assert!(
        profile_prior
            .admit_successor(&profile_next, profile_event.clone(), &profile_intent)
            .is_ok(),
        "benchmark must measure an admitted full Modeline transition"
    );
    criterion.bench_function("ui/keymap/M-x", |bench| {
        bench.iter(|| black_box(&prior).key_command(black_box(&["M-x"])))
    });
    assert_eq!(
        prior.key_command(&["C-n"]),
        Some(UiKeyCommand::Edit(UiEditorAction::InsertNewline))
    );
    criterion.bench_function("ui/keymap/C-n-editor", |bench| {
        bench.iter(|| black_box(&prior).key_command(black_box(&["C-n"])))
    });
    criterion.bench_function("ui/keymap/C-x-prefix", |bench| {
        bench.iter(|| black_box(&prior).key_prefix(black_box(&["C-x"])))
    });
    criterion.bench_function("ui/frame/select-buffer-admission", |bench| {
        bench.iter(|| {
            black_box(&prior).admit_successor(
                black_box(&selected),
                black_box(event.clone()),
                black_box(&intent),
            )
        })
    });
    criterion.bench_function("ui/profile/reordered-modeline-admission", |bench| {
        bench.iter(|| {
            black_box(&profile_prior).admit_successor(
                black_box(&profile_next),
                black_box(profile_event.clone()),
                black_box(&profile_intent),
            )
        })
    });
}

fn configured() -> Criterion {
    Criterion::default()
        .sample_size(30)
        .measurement_time(Duration::from_secs(3))
}

criterion_group! { name = ui_hot_path; config = configured(); targets = benchmark_hot_path }
criterion_main!(ui_hot_path);
