use super::input::{BracketedPaste, InputDecoder, TerminalInput};
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
