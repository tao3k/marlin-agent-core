// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::Command;
use marlin_agent_ui::{UiCommand, UiCommandArguments, UiCommandCapability, UiCommandName};

fn descriptor(
    name: &str,
    arguments: UiCommandArguments,
    capability: UiCommandCapability,
    value: Option<&str>,
) -> UiCommand {
    UiCommand {
        name: UiCommandName::new(name).unwrap(),
        description: String::new(),
        arguments,
        capability,
        value: value.map(str::to_owned),
    }
}

#[test]
fn selected_descriptors_define_names_and_native_capability_arguments() {
    use UiCommandArguments::{None as NoArguments, Optional, Required};
    use UiCommandCapability as Capability;
    let alias = descriptor("choose-fish", Required, Capability::Model, None);
    assert_eq!(
        Command::parse(&alias, "choose-fish  x "),
        Some(Command::Model("x"))
    );
    assert_eq!(Command::parse(&alias, "model x"), None);
    let sessions = descriptor("browse-sessions", Optional, Capability::Sessions, None);
    assert_eq!(
        Command::parse(&sessions, "browse-sessions"),
        Some(Command::Sessions(false))
    );
    assert_eq!(
        Command::parse(&sessions, "browse-sessions next"),
        Some(Command::Sessions(true))
    );
    assert_eq!(Command::parse(&sessions, "browse-sessions other"), None);
    assert_eq!(
        Command::parse(&sessions, "browse-sessions next other"),
        None
    );
    assert!(!Command::admit(&descriptor(
        "legacy-next",
        NoArguments,
        Capability::Sessions,
        Some("next"),
    )));
    let older = descriptor("earlier", NoArguments, Capability::History, Some("older"));
    assert_eq!(
        Command::parse(&older, "earlier"),
        Some(Command::History(true))
    );
    assert!(!Command::admit(&descriptor(
        "invalid-history",
        Required,
        Capability::History,
        None,
    )));
    let buffer = descriptor(
        "show-errors",
        NoArguments,
        Capability::SelectBuffer,
        Some("messages"),
    );
    assert!(matches!(
        Command::parse(&buffer, "show-errors"),
        Some(Command::SelectUi(_))
    ));
    assert_eq!(Command::parse(&buffer, "show-errors extra"), None);
    let invalid = descriptor(
        "forged",
        NoArguments,
        Capability::SelectBuffer,
        Some("unknown!"),
    );
    assert!(!Command::admit(&invalid));
    assert!(!Command::admit(&descriptor(
        "status-with-arguments",
        Optional,
        Capability::Workspace,
        Some("status"),
    )));
    assert!(!Command::admit(&descriptor(
        "diff-without-arguments",
        NoArguments,
        Capability::Workspace,
        Some("diff"),
    )));
    let thought = descriptor("thought", Optional, Capability::ToggleThinking, None);
    assert_eq!(
        Command::parse(&thought, "thought"),
        Some(Command::ToggleThinking(1))
    );
    assert_eq!(
        Command::parse(&thought, "thought 2"),
        Some(Command::ToggleThinking(2))
    );
    assert_eq!(Command::parse(&thought, "thought 0"), None);
    assert_eq!(Command::parse(&thought, "thought -1"), None);
    assert!(!Command::admit(&descriptor(
        "thought-without-arguments",
        NoArguments,
        Capability::ToggleThinking,
        None,
    )));
}
