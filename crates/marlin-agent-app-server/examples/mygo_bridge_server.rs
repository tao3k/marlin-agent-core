// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Runnable Rust `ui-grpc` bridge server for live Go `marlin-mygo` <-> Rust
//! `marlin-agent-core` cross-process HTTP/2 gRPC end-to-end verification.

use std::{collections::BTreeMap, env};

use marlin_agent_app_server::{
    ui_grpc::MyGoBridgeHost,
    wire::marlin::gate::v1::{GateKind, PendingGate},
};
use marlin_agent_runtime::plugin::{PublisherId, ReleaseTrust};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Status, transport::Server};

fn decode_hex_32(hex: &str) -> Result<[u8; 32], String> {
    let trimmed = hex.trim();
    if trimmed.len() != 64 {
        return Err(format!("expected 64 hex chars, got {}", trimmed.len()));
    }
    let mut out = [0u8; 32];
    for (i, slot) in out.iter_mut().enumerate() {
        let byte = u8::from_str_radix(&trimmed[i * 2..i * 2 + 2], 16)
            .map_err(|e| format!("invalid hex at {i}: {e}"))?;
        *slot = byte;
    }
    Ok(out)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bind_addr = env::var("MARLIN_BRIDGE_ADDR").unwrap_or_else(|_| "127.0.0.1:0".to_owned());
    let token = env::var("MARLIN_BRIDGE_TOKEN").unwrap_or_else(|_| "marlin-dev-token".to_owned());
    let publisher_id =
        env::var("MARLIN_BRIDGE_PUBLISHER").unwrap_or_else(|_| "tao3k.official".to_owned());

    let mut keys = BTreeMap::new();
    if let Ok(pub_hex) = env::var("MARLIN_BRIDGE_PUBKEY_HEX") {
        let pub_bytes = decode_hex_32(&pub_hex)?;
        keys.insert(PublisherId::try_from(publisher_id)?, pub_bytes);
    }

    let expected_header = format!("Bearer {token}");
    let host = MyGoBridgeHost::new(ReleaseTrust::new(keys), move |meta| {
        match meta.get("authorization") {
            Some(v) if v.to_str().ok() == Some(expected_header.as_str()) => Ok(()),
            _ => Err(Status::unauthenticated("invalid bearer token")),
        }
    });

    host.enqueue_pending_gate(PendingGate {
        gate_id: "gate-e2e-1".to_owned(),
        kind: GateKind::DeviceCapability as i32,
        turn: None,
        call_id: Some("call-e2e-1".to_owned()),
        tool_or_node_name: "device.ios.notify.send".to_owned(),
        prompt: "Approve device notification delivery on iPhone?".to_owned(),
        details_heading: "platform.ios Notifications".to_owned(),
        choices: Vec::new(),
        card: None,
        created_at_unix_ms: 1,
        expires_at_unix_ms: None,
    });

    let listener = TcpListener::bind(&bind_addr).await?;
    let local_addr = listener.local_addr()?;
    println!("READY {local_addr}");

    Server::builder()
        .add_service(host.app_service())
        .add_service(host.ui_service())
        .add_service(host.gate_service())
        .add_service(host.plugin_service())
        .add_service(host.device_service())
        .add_service(host.projection_service())
        .serve_with_incoming(TcpListenerStream::new(listener))
        .await?;
    Ok(())
}
