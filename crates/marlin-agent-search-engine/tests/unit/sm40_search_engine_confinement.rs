// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! SM40 (ADV-L4) ASP HTTP Patch Confinement, Loopback Transport Integrity,
//! Circular JSON Schema Anti-DoS & Search-Index Cache Isolation tests.

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use marlin_agent_search_engine::{
    AspForwardingEndpoint, AspHttpAstPatchOp, AspHttpAstPatchRequest, HttpEndpointPath,
    TargetedJsonSchemaDocument, WorkspaceIndexConfig, WorkspaceSearchIndex,
    apply_ast_patch_via_asp_http, fetch_asp_provider_catalog,
    hydrate_workspace_open_edb_org_and_cache, save_workspace_snapshot_atomic,
};

#[cfg(unix)]
#[test]
fn sm40_ast_patch_refuses_traversal_absolute_and_symlink_escape() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().join("workspace");
    let outside = dir.path().join("outside");
    fs::create_dir_all(workspace.join("src")).expect("mkdir workspace");
    fs::create_dir_all(&outside).expect("mkdir outside");

    let victim = outside.join("victim.rs");
    fs::write(&victim, "pub fn secret_outside() {}\n").expect("write victim");
    let in_ws = workspace.join("src/safe.rs");
    fs::write(&in_ws, "pub fn safe_before() {}\n").expect("write safe");

    let leaf_link = workspace.join("src/leaf_link.rs");
    std::os::unix::fs::symlink(&victim, &leaf_link).expect("leaf symlink");
    let dir_link = workspace.join("linked_dir");
    std::os::unix::fs::symlink(&outside, &dir_link).expect("dir symlink");

    // Spin up a loopback mock ASP server that echoes back a patched file if reached.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let addr = listener.local_addr().expect("local addr");
    let server = thread::spawn(move || {
        for _ in 0..8 {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(500)));
            let mut buf = [0u8; 8192];
            let n = stream.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]);
            let body = if req.starts_with("GET /v1/providers") {
                r#"{"providers":[]}"#.to_owned()
            } else {
                r#"{"status":"applied","path":"src/safe.rs","languageId":"rust","providerId":"asp-rust","preimageHash":"h1","postimageHash":"h2","patchedSourceUtf8":"pub fn safe_after() {}\n","reparseVerified":true}"#.to_owned()
            };
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
            let _ = stream.shutdown(std::net::Shutdown::Write);
            let mut drain = [0u8; 256];
            while stream.read(&mut drain).unwrap_or(0) > 0 {}
        }
    });

    let endpoint = AspForwardingEndpoint::new(format!("http://{addr}")).expect("loopback endpoint");
    let mut index = WorkspaceSearchIndex::build_with_config(&workspace, WorkspaceIndexConfig::default());
    let abs_victim = victim.to_string_lossy().into_owned();

    for hostile_path in [
        "../outside/victim.rs",
        abs_victim.as_str(),
        "src/leaf_link.rs",
        "linked_dir/victim.rs",
    ] {
        let res = apply_ast_patch_via_asp_http(
            &endpoint,
            &mut index,
            &AspHttpAstPatchRequest {
                workspace_root: &workspace,
                path: hostile_path,
                language_override: None,
                expected_preimage_hash: None,
                operation: AspHttpAstPatchOp::ReplaceItem {
                    target: "secret_outside".to_owned(),
                    snippet: "pub fn pwned() {}".to_owned(),
                },
                dry_run: false,
                auto_calibrate: false,
            },
        );
        assert!(
            res.is_err(),
            "SM40 AST patch must refuse escaping/symlink path {hostile_path:?}, got {res:?}"
        );
    }

    let victim_after = fs::read_to_string(&victim).expect("read victim");
    assert_eq!(
        victim_after, "pub fn secret_outside() {}\n",
        "outside victim file was overwritten by AST patch escape!"
    );

    // Valid confined in-workspace file patch still succeeds cleanly.
    let ok_receipt = apply_ast_patch_via_asp_http(
        &endpoint,
        &mut index,
        &AspHttpAstPatchRequest {
            workspace_root: &workspace,
            path: "src/safe.rs",
            language_override: None,
            expected_preimage_hash: None,
            operation: AspHttpAstPatchOp::ReplaceItem {
                target: "safe_before".to_owned(),
                snippet: "pub fn safe_after() {}".to_owned(),
            },
            dry_run: false,
            auto_calibrate: false,
        },
    );
    assert!(
        ok_receipt.is_ok(),
        "valid confined workspace patch should succeed: {ok_receipt:?}"
    );
    assert_eq!(
        fs::read_to_string(&in_ws).expect("read patched"),
        "pub fn safe_after() {}\n"
    );
    drop(server);
}

#[test]
fn sm40_loopback_endpoint_and_transport_refuse_ssrf_crlf_and_chunked_overflow() {
    // 1. Strict loopback endpoint parsing: reject @-userinfo, port 0, paths, controls, bad port.
    for hostile_url in [
        "http://127.0.0.1:80@evil.example",
        "http://localhost:80@evil.example",
        "http://127.0.0.1:0",
        "http://127.0.0.1:80\r\nX-Injected: 1",
        "http://127.0.0.1:80/extra/path",
        "http://127.0.0.1:notaport",
        "http://127.0.0.1:99999",
    ] {
        assert!(
            AspForwardingEndpoint::new(hostile_url).is_err(),
            "AspForwardingEndpoint::new must reject hostile endpoint {hostile_url:?}"
        );
    }

    // 2. HttpEndpointPath must reject CRLF / whitespace HTTP request smuggling.
    for hostile_path in [
        "/health HTTP/1.1\r\nHost: evil\r\n\r\nPOST /shutdown",
        "/v1/providers\r\nX-Smuggled: 1",
        "/v1/providers?a=1 b=2",
    ] {
        assert!(
            HttpEndpointPath::new(hostile_path).is_err(),
            "HttpEndpointPath::new must reject smuggling path {hostile_path:?}"
        );
    }

    // 3. Control token containing CR/LF must NEVER inject raw HTTP headers on the wire.
    let crlf_listener = TcpListener::bind("127.0.0.1:0").expect("bind crlf listener");
    let crlf_addr = crlf_listener.local_addr().expect("crlf addr");
    let captured_req = thread::spawn(move || {
        let (mut stream, _) = crlf_listener.accept().expect("accept");
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(500)));
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).unwrap_or(0);
        let body = r#"{"providers":[]}"#;
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(resp.as_bytes());
        String::from_utf8_lossy(&buf[..n]).into_owned()
    });
    let injected_ep = AspForwardingEndpoint::new(format!("http://{crlf_addr}"))
        .expect("clean loopback")
        .with_control_token("tok\r\nX-Smuggled: pwned");
    assert!(
        injected_ep.control_token().is_none(),
        "with_control_token must reject CRLF-tainted control tokens"
    );
    let _ = fetch_asp_provider_catalog(&injected_ep);
    let wire_req = captured_req.join().expect("join wire capture");
    assert!(
        !wire_req.contains("X-Smuggled"),
        "CRLF control token smuggled raw HTTP header onto wire: {wire_req:?}"
    );

    // 4. Chunked response with usize::MAX hex size must fail closed without integer-overflow panic.
    let chunk_listener = TcpListener::bind("127.0.0.1:0").expect("bind chunk listener");
    let chunk_addr = chunk_listener.local_addr().expect("chunk addr");
    let chunk_srv = thread::spawn(move || {
        let (mut stream, _) = chunk_listener.accept().expect("accept chunk");
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf);
        let hostile_resp =
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\nffffffffffffffff\r\nhello\r\n0\r\n\r\n";
        let _ = stream.write_all(hostile_resp.as_bytes());
        let _ = stream.flush();
    });
    let chunk_ep = AspForwardingEndpoint::new(format!("http://{chunk_addr}")).expect("chunk ep");
    let outcome = std::panic::catch_unwind(|| fetch_asp_provider_catalog(&chunk_ep));
    let _ = chunk_srv.join();
    assert!(
        matches!(outcome, Ok(Err(_))),
        "hostile usize::MAX chunked size must fail closed cleanly without panic; got {outcome:?}"
    );

    // 5. Truncated chunked response (missing 0\r\n terminator) must fail closed.
    let trunc_listener = TcpListener::bind("127.0.0.1:0").expect("bind trunc listener");
    let trunc_addr = trunc_listener.local_addr().expect("trunc addr");
    let trunc_srv = thread::spawn(move || {
        let (mut stream, _) = trunc_listener.accept().expect("accept trunc");
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf);
        let resp = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n10\r\n{\"providers\":[]}\r\n";
        let _ = stream.write_all(resp.as_bytes());
        let _ = stream.flush();
    });
    let trunc_ep = AspForwardingEndpoint::new(format!("http://{trunc_addr}")).expect("trunc ep");
    let trunc_res = fetch_asp_provider_catalog(&trunc_ep);
    let _ = trunc_srv.join();
    assert!(
        trunc_res.is_err(),
        "truncated chunked body missing 0-chunk terminator must fail closed: {trunc_res:?}"
    );
}

#[test]
fn sm40_circular_json_schema_ref_terminates_without_stack_overflow() {
    let self_cycle: TargetedJsonSchemaDocument = serde_json::from_value(serde_json::json!({
        "type": "object",
        "$defs": {
            "Loop": { "$ref": "#/$defs/Loop" }
        },
        "properties": {
            "kind": { "$ref": "#/$defs/Loop" }
        }
    }))
    .expect("parse self cycle schema");
    assert!(self_cycle.kind_enum().is_empty());
    assert!(self_cycle.property_type("kind").is_none());

    let mutual_cycle: TargetedJsonSchemaDocument = serde_json::from_value(serde_json::json!({
        "type": "object",
        "$defs": {
            "A": { "$ref": "#/$defs/B" },
            "B": { "anyOf": [{ "$ref": "#/$defs/A" }, { "type": "null" }] }
        },
        "properties": {
            "kind": { "$ref": "#/$defs/A" }
        }
    }))
    .expect("parse mutual cycle schema");
    assert!(mutual_cycle.kind_enum().is_empty());

    // Valid non-cyclic $ref chain continues to resolve normally.
    let valid_ref: TargetedJsonSchemaDocument = serde_json::from_value(serde_json::json!({
        "type": "object",
        "$defs": {
            "Kind": { "type": "string", "enum": ["function", "struct"] }
        },
        "properties": {
            "kind": { "anyOf": [{ "$ref": "#/$defs/Kind" }, { "type": "null" }] }
        }
    }))
    .expect("parse valid schema");
    assert_eq!(valid_ref.kind_enum(), vec!["function", "struct"]);
    assert_eq!(valid_ref.property_type("kind"), Some("string"));
}

#[cfg(unix)]
#[test]
fn sm40_snapshot_and_open_edb_cache_refuse_symlinks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().join("ws");
    let outside = dir.path().join("outside");
    let cache_dir = dir.path().join("cache");
    fs::create_dir_all(&workspace).expect("mkdir ws");
    fs::create_dir_all(&outside).expect("mkdir outside");
    fs::create_dir_all(&cache_dir).expect("mkdir cache");

    fs::write(workspace.join("notes.org"), "* SM40 Heading\nBody text\n").expect("write org");

    let victim_snap = outside.join("victim_snap.bin");
    fs::write(&victim_snap, "DO_NOT_OVERWRITE_SNAP").expect("write victim snap");
    let leaf_snap_link = cache_dir.join("snap-link.bin");
    std::os::unix::fs::symlink(&victim_snap, &leaf_snap_link).expect("symlink snap");

    let sym_cache_dir = cache_dir.join("linked-dir");
    std::os::unix::fs::symlink(&outside, &sym_cache_dir).expect("symlink dir");

    let mut index = WorkspaceSearchIndex::build_with_config(&workspace, WorkspaceIndexConfig::default());

    assert!(
        save_workspace_snapshot_atomic(&index, &leaf_snap_link).is_err(),
        "save_workspace_snapshot_atomic must refuse leaf symlink snapshot target"
    );
    assert!(
        save_workspace_snapshot_atomic(&index, &sym_cache_dir.join("escaped.bin")).is_err(),
        "save_workspace_snapshot_atomic must refuse symlinked parent directory"
    );
    assert_eq!(
        fs::read_to_string(&victim_snap).expect("read victim snap"),
        "DO_NOT_OVERWRITE_SNAP"
    );
    assert!(
        !outside.join("escaped.bin").exists(),
        "snapshot must not be created inside symlinked outside directory"
    );

    let victim_cache = outside.join("victim_cache.json");
    fs::write(&victim_cache, "DO_NOT_OVERWRITE_CACHE").expect("write victim cache");
    let leaf_cache_link = cache_dir.join("edb-link.json");
    std::os::unix::fs::symlink(&victim_cache, &leaf_cache_link).expect("symlink cache");

    let receipt = hydrate_workspace_open_edb_org_and_cache(&mut index, &leaf_cache_link);
    assert!(
        !receipt.cache_saved,
        "hydrate_workspace_open_edb_org_and_cache must refuse writing through symlinked cache_path"
    );
    assert_eq!(
        fs::read_to_string(&victim_cache).expect("read victim cache"),
        "DO_NOT_OVERWRITE_CACHE",
        "symlinked Open EDB cache target was overwritten!"
    );
}
