// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::Command;

#[test]
fn parser_preserves_command_boundaries_and_arguments() {
    assert_eq!(Command::parse("/model  x "), Some(Command::Model("x")));
    assert_eq!(Command::parse("/models"), Some(Command::Models));
    assert_eq!(
        Command::parse("/sessions next"),
        Some(Command::Sessions(true))
    );
    assert_eq!(Command::parse("/sessions  next"), None);
    assert_eq!(
        Command::parse("/diff staged \"a b\""),
        Some(Command::Workspace("/diff staged \"a b\""))
    );
    assert_eq!(Command::parse("/status extra"), None);
    assert_eq!(Command::parse("/inspectx \"x\""), None);
}
