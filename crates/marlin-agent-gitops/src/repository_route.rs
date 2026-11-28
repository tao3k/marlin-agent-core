// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Select a Git fetch route and encode its repository path for local storage.

use std::{
    fmt::Write,
    path::{Path, PathBuf},
};

use thiserror::Error;
use url::{Host, Url};

use crate::{
    GitContext, GitOperations, GitReadError,
    read::{LocalExecutor, read_git_with},
};

#[cfg(test)]
#[path = "../tests/unit/repository_route.rs"]
mod tests;

/// Name of a configured Git remote, distinct from its fetch URL.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitRemoteName(String);

impl GitRemoteName {
    pub fn new(name: impl Into<String>) -> Result<Self, RepositoryRouteError> {
        let name = name.into();
        if name.is_empty() || name.starts_with('-') || name.chars().any(char::is_control) {
            return Err(RepositoryRouteError::InvalidRemoteName);
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated relative route below the repository store, shared by matching origins.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkRepositoryRoute(PathBuf);

impl NetworkRepositoryRoute {
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// A route selects a storage namespace. An enrollment still identifies the local clone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepositoryRoute {
    Local,
    Network(NetworkRepositoryRoute),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RepositoryRouteError {
    #[error(transparent)]
    Git(#[from] GitReadError),
    #[error("invalid Git remote name")]
    InvalidRemoteName,
    #[error("the selected Git remote is missing")]
    MissingRemote,
    #[error("the selected Git remote has no fetch URL")]
    MissingFetchUrl,
    #[error("the Git fetch URL cannot identify a repository route")]
    InvalidFetchUrl,
    #[error("the Git remote has distinct fetch routes")]
    ConflictingFetchRoutes,
    #[error("multiple network remotes are eligible; select the source remote")]
    AmbiguousRemotes,
    #[error("the repository has no eligible network remote; select a local remote explicitly")]
    NoEligibleRemote,
}

impl GitOperations {
    /// Resolve the configured source remote, origin, or sole eligible network remote.
    pub async fn repository_route(
        context: &GitContext,
        preferred: Option<&GitRemoteName>,
    ) -> Result<RepositoryRoute, RepositoryRouteError> {
        let root = context.worktree().as_path();
        let output = read_git_with(&LocalExecutor, root, &["remote".into()]).await?;
        let names =
            std::str::from_utf8(&output).map_err(|_| RepositoryRouteError::InvalidRemoteName)?;
        let remotes = names
            .lines()
            .map(GitRemoteName::new)
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(name) = preferred {
            if !remotes.contains(name) {
                return Err(RepositoryRouteError::MissingRemote);
            }
            return read_remote_route(root, name).await;
        }
        if let Some(origin) = remotes.iter().find(|name| name.as_str() == "origin") {
            return read_remote_route(root, origin).await;
        }
        if remotes.is_empty() {
            return Ok(RepositoryRoute::Local);
        }
        let mut selected = None;
        for remote in remotes {
            match read_remote_route(root, &remote).await {
                Ok(route @ RepositoryRoute::Network(_)) if selected.is_none() => {
                    selected = Some(route)
                }
                Ok(RepositoryRoute::Network(_)) => {
                    return Err(RepositoryRouteError::AmbiguousRemotes);
                }
                Ok(RepositoryRoute::Local)
                | Err(
                    RepositoryRouteError::InvalidFetchUrl | RepositoryRouteError::MissingFetchUrl,
                ) => {}
                Err(error) => return Err(error),
            }
        }
        selected.ok_or(RepositoryRouteError::NoEligibleRemote)
    }
}

async fn read_remote_route(
    root: &Path,
    remote: &GitRemoteName,
) -> Result<RepositoryRoute, RepositoryRouteError> {
    // Read the configured count without relying on Git's line-based URL display.
    // An embedded newline must not be interpreted as a second configured URL.
    let name = remote.as_str();
    let configured = match read_git_with(
        &LocalExecutor,
        root,
        &[
            "config".into(),
            "--null".into(),
            "--get-all".into(),
            format!("remote.{name}.url").into(),
        ],
    )
    .await
    {
        Err(GitReadError::Failed(Some(1))) => return Err(RepositoryRouteError::MissingFetchUrl),
        result => result?,
    };
    let raw = std::str::from_utf8(&configured)
        .ok()
        .and_then(|raw| raw.strip_suffix('\0'))
        .filter(|raw| {
            !raw.is_empty()
                && raw
                    .split('\0')
                    .all(|url| !url.bytes().any(|byte| byte.is_ascii_control()))
        })
        .ok_or(RepositoryRouteError::InvalidFetchUrl)?;
    let output = read_git_with(
        &LocalExecutor,
        root,
        &[
            "remote".into(),
            "get-url".into(),
            "--all".into(),
            "--".into(),
            name.into(),
        ],
    )
    .await?;
    let effective = std::str::from_utf8(&output)
        .ok()
        .and_then(|urls| urls.strip_suffix('\n'))
        .ok_or(RepositoryRouteError::InvalidFetchUrl)?;
    let urls = effective.split('\n');
    if urls.clone().count() != raw.split('\0').count() {
        return Err(RepositoryRouteError::InvalidFetchUrl);
    }
    let mut routes = urls.map(fetch_route);
    let route = routes
        .next()
        .ok_or(RepositoryRouteError::MissingFetchUrl)??;
    for candidate in routes {
        if candidate? != route {
            return Err(RepositoryRouteError::ConflictingFetchRoutes);
        }
    }
    Ok(match route {
        FetchRoute::Local(_) => RepositoryRoute::Local,
        FetchRoute::Network(route) => RepositoryRoute::Network(route),
    })
}

#[derive(Debug, Eq, PartialEq)]
enum FetchRoute<'a> {
    Local(&'a str),
    Network(NetworkRepositoryRoute),
}

fn fetch_route(input: &str) -> Result<FetchRoute<'_>, RepositoryRouteError> {
    use RepositoryRouteError::InvalidFetchUrl;
    if input.is_empty() || input.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(InvalidFetchUrl);
    }
    if let Some((scheme, location)) = input.split_once("://") {
        if scheme.eq_ignore_ascii_case("file") {
            let parsed = Url::parse(input).map_err(|_| InvalidFetchUrl)?;
            if parsed.host().is_none() && parsed.query().is_none() && parsed.fragment().is_none() {
                return Ok(FetchRoute::Local(input));
            }
            return Err(InvalidFetchUrl);
        }
        let (authority, path) = location.split_once('/').ok_or(InvalidFetchUrl)?;
        return network_route(scheme, authority, path).map(FetchRoute::Network);
    }
    if Path::new(input).is_absolute() || input.starts_with("./") || input.starts_with("../") {
        return Ok(FetchRoute::Local(input));
    }
    let authority = input.split('/').next().unwrap_or_default();
    if let Some((prefix, _)) = authority.split_once(':') {
        let host = prefix.rsplit('@').next().unwrap_or_default();
        let delimiter = if host.starts_with('[') {
            authority
                .find("]:")
                .map(|close| close + 1)
                .ok_or(InvalidFetchUrl)?
        } else {
            prefix.len()
        };
        let path = &input[delimiter + 1..];
        if path.starts_with(':') {
            return Err(InvalidFetchUrl);
        }
        return network_route(
            "ssh",
            &input[..delimiter],
            path.strip_prefix('/').unwrap_or(path),
        )
        .map(FetchRoute::Network);
    }
    Ok(FetchRoute::Local(input))
}

fn network_route(
    scheme: &str,
    authority: &str,
    path: &str,
) -> Result<NetworkRepositoryRoute, RepositoryRouteError> {
    use RepositoryRouteError::InvalidFetchUrl;
    let scheme = scheme.to_ascii_lowercase();
    let default_port = match scheme.as_str() {
        "http" => 80,
        "https" => 443,
        "ssh" => 22,
        "git" => 9418,
        _ => return Err(InvalidFetchUrl),
    };
    if authority.is_empty()
        || [authority, path].into_iter().any(|part| {
            part.contains(['?', '#', '\\']) || part.bytes().any(|byte| byte.is_ascii_whitespace())
        })
    {
        return Err(InvalidFetchUrl);
    }
    // Parse the authority with the URL library. Decode the original path below
    // so URL dot-segment normalization cannot conceal invalid source input.
    let url = Url::parse(&format!("{scheme}://{authority}/")).map_err(|_| InvalidFetchUrl)?;
    let mut host = Host::parse(url.host_str().ok_or(InvalidFetchUrl)?)
        .map_err(|_| InvalidFetchUrl)?
        .to_string()
        .to_ascii_lowercase();
    if let Some(port) = url.port().filter(|&port| port != default_port) {
        write!(&mut host, ":{port}").expect("writes to String succeed");
    }
    let host = if host == "_local" {
        "%5Flocal".to_owned()
    } else {
        encode_segment(host.as_bytes())?
    };
    let mut route = PathBuf::from(host);
    let path = path.strip_suffix('/').unwrap_or(path);
    let mut segments = path.split('/').peekable();
    while let Some(segment) = segments.next() {
        let bytes = fetch_segment(segment)?;
        let bytes = if segments.peek().is_none() {
            bytes.strip_suffix(b".git").unwrap_or(&bytes)
        } else {
            &bytes
        };
        route.push(encode_segment(bytes)?);
    }
    Ok(NetworkRepositoryRoute(route))
}

fn fetch_segment(input: &str) -> Result<Vec<u8>, RepositoryRouteError> {
    let mut decoded = Vec::with_capacity(input.len());
    let mut bytes = input.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let hex = |byte| char::from(byte).to_digit(16);
            let high = bytes
                .next()
                .and_then(hex)
                .ok_or(RepositoryRouteError::InvalidFetchUrl)?;
            let low = bytes
                .next()
                .and_then(hex)
                .ok_or(RepositoryRouteError::InvalidFetchUrl)?;
            decoded.push((high * 16 + low) as u8);
        } else {
            decoded.push(byte);
        }
    }
    Ok(decoded)
}

fn encode_segment(bytes: &[u8]) -> Result<String, RepositoryRouteError> {
    if bytes.is_empty()
        || matches!(bytes, b"." | b"..")
        || bytes
            .iter()
            .any(|&byte| matches!(byte, b'/' | b'\\') || byte.is_ascii_control())
    {
        return Err(RepositoryRouteError::InvalidFetchUrl);
    }
    let stem = bytes.split(|&byte| byte == b'.').next().unwrap_or_default();
    let reserved_device = matches!(
        stem,
        b"con"
            | b"prn"
            | b"aux"
            | b"nul"
            | [b'c', b'o', b'm', b'1'..=b'9']
            | [b'l', b'p', b't', b'1'..=b'9']
    );
    let mut encoded = String::new();
    for (index, &byte) in bytes.iter().enumerate() {
        let requires_escape =
            (byte == b'.' && index + 1 == bytes.len()) || (reserved_device && index == 0);
        if !requires_escape
            && (byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b'-' | b'~'))
        {
            encoded.push(char::from(byte));
        } else {
            write!(&mut encoded, "%{byte:02X}").expect("writes to String succeed");
        }
    }
    Ok(encoded)
}
