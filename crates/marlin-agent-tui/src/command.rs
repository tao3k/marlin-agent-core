// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed authority for the terminal's built-in commands.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CommandKind {
    Help,
    Models,
    Model,
    Sessions,
    Session,
    Inspect,
    History,
    Status,
    Diff,
    Context,
    Runtime,
    Clear,
}

impl CommandKind {
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "/help" => Self::Help,
            "/models" => Self::Models,
            "/model" => Self::Model,
            "/sessions" => Self::Sessions,
            "/session" => Self::Session,
            "/inspect" => Self::Inspect,
            "/history" => Self::History,
            "/status" => Self::Status,
            "/diff" => Self::Diff,
            "/context" => Self::Context,
            "/runtime" => Self::Runtime,
            "/clear" => Self::Clear,
            _ => return None,
        })
    }

    pub(crate) fn accepts_arguments(self) -> bool {
        matches!(
            self,
            Self::Model
                | Self::Sessions
                | Self::Session
                | Self::Inspect
                | Self::History
                | Self::Diff
        )
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Command<'a> {
    Help,
    Models,
    Model(&'a str),
    Sessions(bool),
    Session(&'a str),
    Inspect(&'a str),
    History(bool),
    Workspace(&'a str),
    Context,
    Runtime,
    Clear,
}

impl<'a> Command<'a> {
    pub(crate) fn parse(input: &'a str) -> Option<Self> {
        let input = input.trim();
        let (name, arg) = input
            .split_once(' ')
            .map_or((input, None), |(name, arg)| (name, Some(arg)));
        let no_args = || arg.is_none().then_some(());
        Some(match CommandKind::from_name(name)? {
            CommandKind::Help => {
                no_args()?;
                Self::Help
            }
            CommandKind::Models => {
                no_args()?;
                Self::Models
            }
            CommandKind::Model => Self::Model(arg?.trim()),
            CommandKind::Sessions => match arg {
                None => Self::Sessions(false),
                Some("next") => Self::Sessions(true),
                _ => return None,
            },
            CommandKind::Session => Self::Session(arg?.trim()),
            CommandKind::Inspect => Self::Inspect(arg?.trim()),
            CommandKind::History => match arg {
                Some("older") => Self::History(true),
                Some("newest") => Self::History(false),
                _ => return None,
            },
            CommandKind::Status => {
                no_args()?;
                Self::Workspace(input)
            }
            CommandKind::Diff => Self::Workspace(input),
            CommandKind::Context => {
                no_args()?;
                Self::Context
            }
            CommandKind::Runtime => {
                no_args()?;
                Self::Runtime
            }
            CommandKind::Clear => {
                no_args()?;
                Self::Clear
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
