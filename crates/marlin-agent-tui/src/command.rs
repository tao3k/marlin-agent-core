// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Native capability admission for Scheme-declared commands.
use marlin_agent_ui::{
    UiBufferId, UiCommand, UiCommandArguments, UiCommandCapability, UiCommandName, UiEvent,
    UiProfileName, UiTransientId,
};

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Command<'a> {
    Models,
    Model(&'a str),
    Sessions(bool),
    Session(&'a str),
    Inspect(&'a str),
    History(bool),
    Workspace {
        mode: &'a str,
        arguments: Option<&'a str>,
    },
    ContextBudget,
    Runtime,
    KillProcess(std::num::NonZeroU64),
    OpenMenu(&'a str),
    SelectUi(UiEvent),
    EnterTransient(UiTransientId),
    ToggleThinking(usize),
    Clear,
}

impl<'a> Command<'a> {
    /// Check the fixed host capability contract, independent of display name.
    pub(crate) fn admit(descriptor: &UiCommand) -> bool {
        use UiCommandArguments::{None as NoArguments, Optional, Required};
        use UiCommandCapability as Capability;
        match (
            descriptor.capability,
            descriptor.arguments,
            descriptor.value.as_deref(),
        ) {
            (
                Capability::Models
                | Capability::ContextBudget
                | Capability::Runtime
                | Capability::Clear,
                NoArguments,
                None,
            ) => true,
            (
                Capability::Model
                | Capability::Session
                | Capability::Inspect
                | Capability::KillProcess,
                Required,
                None,
            ) => true,
            (Capability::Sessions, Optional, None) => true,
            (Capability::History, NoArguments, Some("older" | "newest")) => true,
            (Capability::Workspace, NoArguments, Some("status"))
            | (Capability::Workspace, Optional, Some("diff"))
            | (Capability::ToggleThinking, Optional, None) => true,
            (Capability::OpenMenu, NoArguments, Some(prefix)) => {
                prefix.ends_with('-') && UiCommandName::is_valid_prefix(prefix)
            }
            (Capability::SelectProfile, NoArguments, Some(profile)) => {
                UiProfileName::new(profile).is_ok()
            }
            (Capability::SelectBuffer, NoArguments, Some(buffer)) => {
                UiBufferId::new(buffer).is_ok()
            }
            (Capability::EnterTransient, NoArguments, Some(id)) => UiTransientId::new(id).is_ok(),
            _ => false,
        }
    }

    /// Parse arguments using the selected descriptor, then select one native
    /// capability. A new command alias needs no Rust name-table edit.
    pub(crate) fn parse(descriptor: &'a UiCommand, input: &'a str) -> Option<Self> {
        if !Self::admit(descriptor) {
            return None;
        }
        let input = input.trim();
        let (name, argument) = input
            .split_once(' ')
            .map_or((input, None), |(name, value)| (name, Some(value)));
        if name != descriptor.name.as_str() {
            return None;
        }
        let argument = match descriptor.arguments {
            UiCommandArguments::None if argument.is_none() => None,
            UiCommandArguments::Required
                if argument.is_some_and(|value| !value.trim().is_empty()) =>
            {
                argument
            }
            UiCommandArguments::Optional if argument.is_none_or(|value| !value.is_empty()) => {
                argument
            }
            _ => return None,
        };
        let value = descriptor.value.as_deref();
        use UiCommandCapability as Capability;
        Some(match descriptor.capability {
            Capability::Models => Self::Models,
            Capability::Model => Self::Model(argument?.trim()),
            Capability::Sessions => Self::Sessions(match argument.map(str::trim) {
                None => false,
                Some("next") => true,
                _ => return None,
            }),
            Capability::Session => Self::Session(argument?.trim()),
            Capability::Inspect => Self::Inspect(argument?.trim()),
            Capability::History => Self::History(value == Some("older")),
            Capability::Workspace => Self::Workspace {
                mode: value?,
                arguments: argument,
            },
            Capability::ContextBudget => Self::ContextBudget,
            Capability::Runtime => Self::Runtime,
            Capability::KillProcess => Self::KillProcess(argument?.trim().parse().ok()?),
            Capability::OpenMenu => Self::OpenMenu(value?),
            Capability::SelectProfile => {
                Self::SelectUi(UiEvent::SelectProfile(UiProfileName::new(value?).ok()?))
            }
            Capability::SelectBuffer => {
                Self::SelectUi(UiEvent::SelectBuffer(UiBufferId::new(value?).ok()?))
            }
            Capability::EnterTransient => Self::EnterTransient(UiTransientId::new(value?).ok()?),
            Capability::Clear => Self::Clear,
            Capability::ToggleThinking => Self::ToggleThinking(match argument {
                Some(value) => value.trim().parse().ok().filter(|index| *index > 0)?,
                None => 1,
            }),
        })
    }
}

#[cfg(test)]
#[path = "../tests/unit/command_source.rs"]
mod tests;
