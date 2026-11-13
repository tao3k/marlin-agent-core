// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::input::{BracketedPaste, InputDecoder, MouseMotion, TerminalInput, WheelAccumulator};
use std::io::Cursor;
use termion::{
    event::{Event, Key},
    input::TermReadEventsAndRaw,
};

#[test]
fn bracketed_multiline_paste_is_one_input_without_enter_or_control_actions() {
    let bytes = "\x1b[200~你好\r\nline\t2\x11\x1b[201~".as_bytes();
    let mut decoder = InputDecoder::default();
    let input: Vec<_> = Cursor::new(bytes)
        .events_and_raw()
        .filter_map(|event| {
            let (event, raw) = event.unwrap();
            decoder.accept(event, &raw)
        })
        .collect();
    assert!(matches!(&input[..], [TerminalInput::Paste(value)] if value == "你好\nline\t2\x11"));
}

#[test]
fn raw_ctrl_c_is_a_key_but_pasted_ctrl_c_is_data() {
    let mut decoder = InputDecoder::default();
    let inputs: Vec<_> = Cursor::new(b"\x03\x1b[200~\x03\x1b[201~")
        .events_and_raw()
        .filter_map(|result| {
            let (event, raw) = result.unwrap();
            decoder.accept(event, &raw)
        })
        .collect();
    assert!(
        matches!(&inputs[..], [TerminalInput::Key(Key::Ctrl('c')), TerminalInput::Paste(text)] if text == "\u{3}")
    );
}

#[test]
fn application_cursor_arrows_are_normalized_without_admitting_other_ss3_sequences() {
    let mut decoder = InputDecoder::default();
    let inputs: Vec<_> = Cursor::new(b"\x1bOA\x1bOB\x1bOP")
        .events_and_raw()
        .filter_map(|result| {
            let (event, raw) = result.unwrap();
            decoder.accept(event, &raw)
        })
        .collect();
    assert!(matches!(
        &inputs[..],
        [
            TerminalInput::Key(Key::Up),
            TerminalInput::Key(Key::Down),
            TerminalInput::Key(Key::F(1))
        ]
    ));
    assert!(
        decoder
            .accept(Event::Unsupported(Vec::new()), b"\x1bOC")
            .is_none()
    );
}

#[test]
fn explicitly_unmodified_csi_arrows_are_normalized_without_accepting_modified_keys() {
    let mut decoder = InputDecoder::default();
    let inputs: Vec<_> = Cursor::new(b"\x1b[1;1A\x1b[1;1B\x1b[1;2A")
        .events_and_raw()
        .filter_map(|result| {
            let (event, raw) = result.unwrap();
            decoder.accept(event, &raw)
        })
        .collect();
    assert!(matches!(
        &inputs[..],
        [
            TerminalInput::Key(Key::Up),
            TerminalInput::Key(Key::Down),
            TerminalInput::Key(Key::ShiftUp)
        ]
    ));
}

#[test]
fn oversized_paste_is_rejected_as_a_whole() {
    let mut decoder = InputDecoder::default();
    assert!(
        decoder
            .accept(Event::Unsupported(Vec::new()), b"\x1b[200~")
            .is_none()
    );
    assert!(
        decoder
            .accept(Event::Key(Key::Char('a')), &vec![b'a'; 1024 * 1024 + 1])
            .is_none()
    );
    assert!(matches!(
        decoder.accept(Event::Unsupported(Vec::new()), b"\x1b[201~"),
        Some(TerminalInput::PasteRejected)
    ));
    assert!(matches!(
        decoder.accept(Event::Key(Key::Char('x')), b"x"),
        Some(TerminalInput::Key(Key::Char('x')))
    ));
}

#[test]
fn bracketed_paste_mode_is_disabled_when_guard_drops() {
    let mut output = Vec::new();
    {
        let _guard = BracketedPaste::enable(&mut output).unwrap();
    }
    assert_eq!(output, b"\x1b[?2004h\x1b[?2004l");
}

#[test]
fn sgr_mouse_motion_is_bounded_and_does_not_enter_the_key_queue() {
    let mut decoder = InputDecoder::default();
    let input: Vec<_> = Cursor::new(b"\x1b[<35;24;8M")
        .events_and_raw()
        .filter_map(|event| {
            let (event, raw) = event.unwrap();
            decoder.accept(event, &raw)
        })
        .collect();
    assert!(matches!(&input[..], [TerminalInput::MouseMove(24, 8)]));
    assert!(
        decoder
            .accept(Event::Unsupported(Vec::new()), b"\x1b[<35;0;8M")
            .is_none()
    );
}

#[test]
fn sgr_wheel_events_reach_the_scroll_queue_with_coordinates() {
    let mut decoder = InputDecoder::default();
    let input: Vec<_> = Cursor::new(b"\x1b[<64;24;8M\x1b[<65;24;8M")
        .events_and_raw()
        .filter_map(|event| {
            let (event, raw) = event.unwrap();
            decoder.accept(event, &raw)
        })
        .collect();
    assert!(matches!(
        &input[..],
        [
            TerminalInput::MouseWheel {
                up: true,
                column: 24,
                row: 8,
            },
            TerminalInput::MouseWheel {
                up: false,
                column: 24,
                row: 8,
            }
        ]
    ));
}

#[test]
fn trackpad_pulses_coalesce_without_blocking_keyboard_input() {
    let wheel = WheelAccumulator::default();
    assert!(wheel.push(true));
    for _ in 0..100 {
        assert!(!wheel.push(true));
    }
    assert_eq!(wheel.take_lines(), Some((true, 48)));
    assert_eq!(wheel.take_lines(), None);
    assert!(wheel.push(false));
    assert_eq!(wheel.take_lines(), Some((false, 3)));
    assert!(wheel.push(true));
    assert!(!wheel.push(false));
    assert_eq!(wheel.take_lines(), None);
}

#[test]
fn mouse_motion_mode_is_disabled_when_guard_drops() {
    let mut output = Vec::new();
    {
        let _guard = MouseMotion::enable(&mut output).unwrap();
    }
    assert_eq!(output, b"\x1b[?1003h\x1b[?1006h\x1b[?1006l\x1b[?1003l");
}
