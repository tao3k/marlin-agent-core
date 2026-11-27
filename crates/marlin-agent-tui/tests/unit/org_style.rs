// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{style_structured_org_line, tone_color};
use marlin_agent_ui::{UiRgb, UiTheme, UiTone};
use ratatui::style::{Modifier, Style};

fn test_theme() -> UiTheme {
    UiTheme {
        canvas: UiRgb {
            red: 10,
            green: 12,
            blue: 16,
        },
        bar: UiRgb {
            red: 24,
            green: 28,
            blue: 36,
        },
        ink: UiRgb {
            red: 220,
            green: 225,
            blue: 235,
        },
        muted: UiRgb {
            red: 120,
            green: 130,
            blue: 145,
        },
        accent: UiRgb {
            red: 180,
            green: 120,
            blue: 240,
        },
        info: UiRgb {
            red: 100,
            green: 180,
            blue: 255,
        },
        success: UiRgb {
            red: 100,
            green: 220,
            blue: 140,
        },
        warning: UiRgb {
            red: 245,
            green: 190,
            blue: 90,
        },
    }
}

#[test]
fn org_line_styler_formats_headings_tables_drawers_checkboxes_and_inline_code() {
    let theme = test_theme();
    let base = Style::default().fg(tone_color(UiTone::Normal, &theme));

    let heading = style_structured_org_line("* TODO Inspect EDB", base, &theme);
    assert!(heading
        .iter()
        .any(|s| s.content == "TODO " && s.style.add_modifier.contains(Modifier::BOLD)));
    assert_eq!(
        heading
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<String>(),
        "* TODO Inspect EDB"
    );

    let block = style_structured_org_line("#+BEGIN_SRC rust", base, &theme);
    assert_eq!(block.len(), 1);
    assert_eq!(block[0].content, "#+BEGIN_SRC rust");
    assert!(block[0].style.bg.is_some());

    let drawer = style_structured_org_line(":PROPERTIES:", base, &theme);
    assert_eq!(drawer[0].content, ":PROPERTIES:");

    let table = style_structured_org_line("| ColA | `val` |", base, &theme);
    let rendered: String = table.iter().map(|s| s.content.as_ref()).collect();
    assert!(rendered.contains("│ ColA │ `val` │"), "{rendered}");

    let checked =
        style_structured_org_line("- [X] Completed [[https://example.com][Docs]]", base, &theme);
    let checked_text: String = checked.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(checked_text, "- [X] Completed Docs");
}
