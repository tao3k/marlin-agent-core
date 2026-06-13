//! Action names for the Gerbil dependency bootstrap CLI.

/// Supported Gerbil dependency bootstrap actions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GerbilDepsAction {
    Env,
    Repair,
    Fetch,
    Link,
    Build,
    Verify,
    Bootstrap,
}

impl GerbilDepsAction {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value {
            "env" => Some(Self::Env),
            "repair" => Some(Self::Repair),
            "fetch" => Some(Self::Fetch),
            "link" => Some(Self::Link),
            "build" => Some(Self::Build),
            "verify" => Some(Self::Verify),
            "bootstrap" => Some(Self::Bootstrap),
            _ => None,
        }
    }
}
