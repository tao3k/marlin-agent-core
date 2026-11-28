// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Loopback HTTP/1.1 client calls for ASP's forwarding gateway.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::SearchPlanProjectionError;

use super::domain_types::AspForwardingEndpoint;
use super::protocol_types::{
    DynamicProviderRegistrationAck, DynamicProviderRegistrationRequest, ForwardedProviderCatalog,
};

const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const HTTP_IO_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_HTTP_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

fn connect_loopback_stream(authority: &str) -> Result<TcpStream, SearchPlanProjectionError> {
    let addrs = authority
        .to_socket_addrs()
        .map_err(|e| SearchPlanProjectionError::Native(format!("resolve ASP HTTP: {e}")))?;
    let mut last_err = None;
    let mut saw_loopback = false;
    for addr in addrs {
        if !addr.ip().is_loopback() {
            continue;
        }
        saw_loopback = true;
        match TcpStream::connect_timeout(&addr, HTTP_CONNECT_TIMEOUT) {
            Ok(stream) => return Ok(stream),
            Err(e) => last_err = Some(e),
        }
    }
    if !saw_loopback {
        return Err(SearchPlanProjectionError::Invalid(
            "ASP endpoint resolved only to non-loopback addresses",
        ));
    }
    let detail = last_err
        .map(|e| e.to_string())
        .unwrap_or_else(|| "no reachable loopback address".to_owned());
    Err(SearchPlanProjectionError::Native(format!(
        "connect ASP HTTP: {detail}"
    )))
}

pub(super) fn http_loopback_request(
    endpoint: &AspForwardingEndpoint,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
) -> Result<(u16, Vec<u8>), SearchPlanProjectionError> {
    if method.is_empty()
        || method.chars().any(|c| c.is_control() || c.is_whitespace())
        || !path.starts_with('/')
        || path.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(SearchPlanProjectionError::Invalid(
            "invalid ASP HTTP method or route path",
        ));
    }
    if endpoint
        .control_token()
        .is_some_and(|tok| tok.chars().any(char::is_control))
    {
        return Err(SearchPlanProjectionError::Invalid(
            "invalid control token containing control characters",
        ));
    }
    let mut stream = connect_loopback_stream(endpoint.authority())?;
    let _ = stream.set_read_timeout(Some(HTTP_IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(HTTP_IO_TIMEOUT));
    let _ = stream.set_nodelay(true);

    let payload = body.unwrap_or(&[]);
    let auth_header = endpoint
        .control_token()
        .map_or_else(String::new, |token| format!("Authorization: Bearer {token}\r\n"));
    let header = format!(
        "{method} {path} HTTP/1.1\r\nHost: {}\r\n{auth_header}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        endpoint.authority(),
        payload.len()
    );
    stream
        .write_all(header.as_bytes())
        .and_then(|()| stream.write_all(payload))
        .and_then(|()| stream.flush())
        .map_err(|e| SearchPlanProjectionError::Native(format!("write ASP HTTP: {e}")))?;

    let mut raw = Vec::new();
    let cap = (MAX_HTTP_RESPONSE_BYTES as u64).saturating_add(1);
    stream
        .take(cap)
        .read_to_end(&mut raw)
        .map_err(|e| SearchPlanProjectionError::Native(format!("read ASP HTTP: {e}")))?;
    if raw.len() > MAX_HTTP_RESPONSE_BYTES {
        return Err(SearchPlanProjectionError::Native(
            "ASP HTTP response exceeded maximum byte cap".to_owned(),
        ));
    }

    let Some(split) = raw.windows(4).position(|w| w == b"\r\n\r\n") else {
        return Err(SearchPlanProjectionError::Native(
            "malformed HTTP response from ASP gateway".to_owned(),
        ));
    };
    let head = std::str::from_utf8(&raw[..split])
        .map_err(|e| SearchPlanProjectionError::Native(format!("HTTP header UTF-8: {e}")))?;
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| SearchPlanProjectionError::Native("missing HTTP status code".to_owned()))?;
    let resp_body = decode_http_body(head, &raw[split + 4..])?;
    Ok((status, resp_body))
}

fn decode_http_body(headers: &str, body: &[u8]) -> Result<Vec<u8>, SearchPlanProjectionError> {
    let is_chunked = headers.lines().any(|line| {
        let lower = line.to_ascii_lowercase();
        lower.starts_with("transfer-encoding:") && lower.contains("chunked")
    });
    if !is_chunked {
        return Ok(body.to_vec());
    }
    let mut out = Vec::new();
    let mut pos = 0usize;
    let mut saw_terminator = false;
    while pos < body.len() {
        let Some(rel_end) = body[pos..].windows(2).position(|w| w == b"\r\n") else {
            return Err(SearchPlanProjectionError::Native(
                "malformed HTTP chunk header".to_owned(),
            ));
        };
        let header_end = pos.checked_add(rel_end).ok_or_else(|| {
            SearchPlanProjectionError::Native("chunk header offset overflow".to_owned())
        })?;
        let size_hex = std::str::from_utf8(&body[pos..header_end])
            .map_err(|e| SearchPlanProjectionError::Native(format!("chunk size UTF-8: {e}")))?
            .split(';')
            .next()
            .unwrap_or("")
            .trim();
        let size = usize::from_str_radix(size_hex, 16)
            .map_err(|e| SearchPlanProjectionError::Native(format!("chunk size hex: {e}")))?;
        pos = header_end.checked_add(2).ok_or_else(|| {
            SearchPlanProjectionError::Native("chunk offset overflow".to_owned())
        })?;
        if size == 0 {
            saw_terminator = true;
            break;
        }
        let chunk_end = pos.checked_add(size).ok_or_else(|| {
            SearchPlanProjectionError::Native("chunk size overflow".to_owned())
        })?;
        let next_pos = chunk_end.checked_add(2).ok_or_else(|| {
            SearchPlanProjectionError::Native("chunk terminator offset overflow".to_owned())
        })?;
        if next_pos > body.len() || &body[chunk_end..next_pos] != b"\r\n" {
            return Err(SearchPlanProjectionError::Native(
                "truncated or malformed HTTP chunked body".to_owned(),
            ));
        }
        if out.len().checked_add(size).is_none_or(|n| n > MAX_HTTP_RESPONSE_BYTES) {
            return Err(SearchPlanProjectionError::Native(
                "HTTP chunked body exceeded maximum byte cap".to_owned(),
            ));
        }
        out.extend_from_slice(&body[pos..chunk_end]);
        pos = next_pos;
    }
    if !saw_terminator {
        return Err(SearchPlanProjectionError::Native(
            "truncated HTTP chunked stream missing terminator".to_owned(),
        ));
    }
    Ok(out)
}

/// Fetch the live provider forwarding catalog (`manifest` + `targetedSchema` + `dslPack`)
/// from ASP `GET /v1/providers`.
pub fn fetch_asp_provider_catalog(
    endpoint: &AspForwardingEndpoint,
) -> Result<ForwardedProviderCatalog, SearchPlanProjectionError> {
    let (status, body) = http_loopback_request(endpoint, "GET", "/v1/providers", None)?;
    if status != 200 {
        return Err(SearchPlanProjectionError::Native(format!(
            "ASP GET /v1/providers returned HTTP {status}: {}",
            String::from_utf8_lossy(&body)
        )));
    }
    serde_json::from_slice(&body)
        .map_err(|e| SearchPlanProjectionError::Native(format!("decode ASP provider catalog: {e}")))
}

/// Dynamically register a new or updated language provider on ASP `POST /v1/providers/register`.
pub fn register_dynamic_provider_on_asp(
    endpoint: &AspForwardingEndpoint,
    request: &DynamicProviderRegistrationRequest,
) -> Result<DynamicProviderRegistrationAck, SearchPlanProjectionError> {
    let encoded = serde_json::to_vec(request).map_err(|e| {
        SearchPlanProjectionError::Native(format!("encode provider registration: {e}"))
    })?;
    let (status, body) =
        http_loopback_request(endpoint, "POST", "/v1/providers/register", Some(&encoded))?;
    if status != 200 {
        return Err(SearchPlanProjectionError::Native(format!(
            "ASP POST /v1/providers/register returned HTTP {status}: {}",
            String::from_utf8_lossy(&body)
        )));
    }
    serde_json::from_slice(&body).map_err(|e| {
        SearchPlanProjectionError::Native(format!("decode ASP register response: {e}"))
    })
}

/// Request graceful shutdown of a loopback ASP forwarding gateway.
pub fn shutdown_asp_gateway(
    endpoint: &AspForwardingEndpoint,
) -> Result<(), SearchPlanProjectionError> {
    let _ = http_loopback_request(endpoint, "POST", "/shutdown", Some(b"{}"));
    Ok(())
}
