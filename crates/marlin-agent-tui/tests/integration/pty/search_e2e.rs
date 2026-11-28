// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Real Unix PTY end-to-end proof of the full Marlin Search + Dusa + ASP + LLM stack:
//! 1. Launches the compiled AOT-linked `marlin-tui` binary inside a real PTY with
//!    auto-supervised `asp-gateway` (`asp-rust` syn + `asp-gerbil-scheme`).
//! 2. Verifies interactive `/helm`: `ASP:5p·` live hydration badge and `Ctrl-t`
//!    preset cycling across L0 Brute-Force -> Case A Ascent -> Case B.1 Dusa
//!    (`HELM-DUSA`, `2 worlds (SAT)`, `W#1·deduction` / `W#1·branch`).
//! 3. Verifies real LLM tool-calling over loopback OpenAI Responses SSE:
//!    - Confirms `workspace_search` schema advertises L0 `(brute-search ...)`,
//!      Case A `(ascent-fragment ...)`, and Case B `(search-choice-program ...)`.
//!    - Executes 3 live sequential LLM tool calls (L0 -> Case A -> Case B.1 Dusa),
//!      approving each in the TUI review menu (`Enter`), and asserts grounded
//!      witness receipts (`mode: brute-force`, `mode: ascent`, `worlds: 2 (SAT`,
//!      `[deduction: closed`, `[branch: open`, zero `mock_parser.rs` leakage)
//!      are delivered back to the LLM.
//! 4. Exits with `Ctrl-q` and verifies terminal restoration.

use super::{StopOnDrop, Terminal, accept_until, read_request, screen_lines};
use std::{
    fs,
    io::Write,
    net::TcpListener,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

fn discover_sibling_asp_gateway() -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os("MARLIN_ASP_GATEWAY_BIN") {
        let candidate = PathBuf::from(explicit);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let marlin_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let asp_root = marlin_root.join("../agent-semantic-protocols");
    for rel in ["target/release/asp-gateway", "target/debug/asp-gateway"] {
        let candidate = asp_root.join(rel);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn write_sse_tool_call(
    stream: &mut std::net::TcpStream,
    resp_id: &str,
    call_id: &str,
    query: &str,
) {
    let args = serde_json::json!({ "query": query }).to_string();
    let response = serde_json::json!({
        "type": "response.completed",
        "response": {
            "status": "completed",
            "id": resp_id,
            "model": "gpt-fixture",
            "output": [{
                "type": "function_call",
                "id": format!("fc-{call_id}"),
                "call_id": call_id,
                "name": "workspace_search",
                "arguments": args,
                "status": "completed"
            }]
        }
    });
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {response}\n\n"
    )
    .unwrap();
}

fn write_sse_final_answer(stream: &mut std::net::TcpStream, answer: &str) {
    let delta = serde_json::json!({
        "type": "response.output_text.delta",
        "delta": answer
    });
    let response = serde_json::json!({
        "type": "response.completed",
        "response": {
            "status": "completed",
            "id": "resp-final",
            "model": "gpt-fixture",
            "output": [{
                "type": "message",
                "content": [{ "type": "output_text", "text": answer }]
            }]
        }
    });
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {delta}\n\ndata: {response}\n\n"
    )
    .unwrap();
}

fn stage_polyglot_lab_workspace(ws: &Path) {
    // Initialize a real Git repository + Cargo.toml & scheme/gerbil.pkg package
    // manifests so ASP `projectMarkers` anchor discovery AND WorkspaceArtifacts
    // (`MARLIN_AGENT_CORE_DEV=1` -> `.data/marlin/`) execute the exact same path
    // as launching `./target/debug/marlin-tui crates/marlin-agent-tui/example.toml`!
    let status = std::process::Command::new("git")
        .current_dir(ws)
        .args(["init", "-q"])
        .status()
        .expect("git init");
    assert!(status.success(), "git init failed");
    fs::create_dir_all(ws.join("crates/marlin-agent-search-engine/src/tests")).unwrap();
    fs::create_dir_all(ws.join("scheme/search-dsl")).unwrap();
    fs::write(
        ws.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/marlin-agent-search-engine\"]\n",
    )
    .unwrap();
    fs::write(
        ws.join("crates/marlin-agent-search-engine/Cargo.toml"),
        "[package]\nname = \"marlin-agent-search-engine\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(ws.join("scheme/gerbil.pkg"), "(package: lab)\n").unwrap();
    fs::write(
        ws.join("crates/marlin-agent-search-engine/src/app.rs"),
        "pub fn marlin_search_engine() -> usize { 7 }\n\
         pub fn local_init() -> usize { 1 }\n\
         pub fn run_app() -> usize {\n\
             let a = local_init();\n\
             let b = parse_payload();\n\
             a + b + marlin_search_engine()\n\
         }\n",
    )
    .unwrap();
    fs::write(
        ws.join("crates/marlin-agent-search-engine/src/parser_a.rs"),
        "pub fn parse_payload() -> usize { 10 }\n",
    )
    .unwrap();
    fs::write(
        ws.join("crates/marlin-agent-search-engine/src/parser_b.rs"),
        "pub fn parse_payload() -> usize { 20 }\n",
    )
    .unwrap();
    fs::write(
        ws.join("crates/marlin-agent-search-engine/src/tests/mock_parser.rs"),
        "pub fn parse_payload() -> usize { 99 }\n",
    )
    .unwrap();
    fs::write(
        ws.join("scheme/search-dsl/funcs.ss"),
        "(def (eval-query x) x)\n",
    )
    .unwrap();
    for args in [
        &["config", "user.email", "lab@marlin.local"][..],
        &["config", "user.name", "Marlin Lab"][..],
        &["add", "."][..],
        &["commit", "-q", "-m", "initial lab workspace"][..],
    ] {
        let status = std::process::Command::new("git")
            .current_dir(ws)
            .args(args)
            .status()
            .expect("git command");
        assert!(status.success(), "git {args:?} failed");
    }
}

#[test]
#[ignore = "real PTY + live asp-gateway + AOT Scheme Dusa + LLM workspace_search E2E"]
fn workspace_search_llm_tool_call_and_helm_dusa_lab_e2e() {
    let Some(asp_bin) = discover_sibling_asp_gateway() else {
        panic!(
            "missing sibling asp-gateway binary; build `cargo build -p agent-semantic-ast --bin asp-gateway`"
        );
    };

    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path();
    stage_polyglot_lab_workspace(ws);

    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_on_drop = StopOnDrop(stop.clone());
    let server_stop = stop.clone();

    let server = thread::spawn(move || {
        // Turn 1 (L0 Brute-Force): verify upgraded S5 workspace_search tool schema,
        // call L0 (brute-search ...), verify receipt, and answer.
        let mut s1 = accept_until(&listener, &server_stop).expect("turn 1 req");
        let (_, req1) = read_request(&mut s1);
        let tools = req1["tools"].as_array().expect("tools array");
        let search_tool = tools
            .iter()
            .find(|t| t["name"] == "workspace_search")
            .expect("workspace_search tool advertised to LLM");
        let desc = search_tool["description"].as_str().expect("description");
        assert!(
            desc.contains("(brute-search ...)"),
            "missing L0 in schema: {desc}"
        );
        assert!(
            desc.contains("(ascent-fragment ...)"),
            "missing Case A in schema: {desc}"
        );
        assert!(
            desc.contains("(search-choice-program ...)"),
            "missing Case B in schema: {desc}"
        );
        assert!(
            desc.contains("(solve-resolve ") && desc.contains("(find-impact "),
            "missing unified L1/L2 macros in schema: {desc}"
        );
        write_sse_tool_call(
            &mut s1,
            "resp-l0",
            "call-l0",
            "(brute-search \"parse_payload\" path-prefix: \"crates/\" lang: \"rust\")",
        );

        let mut s1_cont = accept_until(&listener, &server_stop).expect("turn 1 cont");
        let (_, req1_cont) = read_request(&mut s1_cont);
        let out_l0 = req1_cont["input"].as_array().unwrap().last().unwrap()["output"]
            .as_str()
            .expect("L0 output");
        assert!(out_l0.contains("mode: brute-force"), "L0 output: {out_l0}");
        assert!(
            out_l0.contains("app.rs"),
            "L0 output missing app.rs: {out_l0}"
        );
        assert!(
            out_l0.contains("[evidence_frontier |")
                && out_l0.contains("teaching_kit: active")
                && out_l0.contains("dsl_tips:")
                && out_l0.contains("workspace_layers:"),
            "L0 Turn-1 missing gRPC Recency-Peak self-teaching frontier: {out_l0}"
        );
        eprintln!("\n[PTY LLM TURN 1 — L0 BRUTE-FORCE RECEIPT]\n{out_l0}");
        write_sse_final_answer(
            &mut s1_cont,
            "L0-OK: located parse_payload via brute-force.",
        );

        // Turn 2 (Case A Structural Ascent): call (ascent-fragment ...), verify receipt, answer.
        let mut s2 = accept_until(&listener, &server_stop).expect("turn 2 req");
        let _ = read_request(&mut s2);
        write_sse_tool_call(
            &mut s2,
            "resp-l1",
            "call-l1",
            "(ascent-fragment ((answer path name line) <-- (file f \"rust\" path) (rust_fn _ f name line)))",
        );

        let mut s2_cont = accept_until(&listener, &server_stop).expect("turn 2 cont");
        let (_, req2_cont) = read_request(&mut s2_cont);
        let out_l1 = req2_cont["input"].as_array().unwrap().last().unwrap()["output"]
            .as_str()
            .expect("L1 output");
        assert!(out_l1.contains("mode: ascent"), "L1 output: {out_l1}");
        assert!(
            out_l1.contains("local_init"),
            "L1 output missing local_init: {out_l1}"
        );
        assert!(
            out_l1.contains("parse_payload"),
            "L1 output missing parse_payload: {out_l1}"
        );
        assert!(
            out_l1.contains("compact_mode: true") && !out_l1.contains("dsl_tips:"),
            "L1 Turn-2 expected compact_mode auto-unloading over gRPC session: {out_l1}"
        );
        eprintln!("\n[PTY LLM TURN 2 — CASE A ASCENT RECEIPT]\n{out_l1}");
        write_sse_final_answer(
            &mut s2_cont,
            "CASE-A-OK: enumerated all Rust fn definitions.",
        );

        // Turn 3 (Case B.1 Dusa Finite-Choice Disambiguation verbatim): call (search-choice-program ...),
        // verify 2 SAT worlds + [deduction: closed] + [branch: open] + zero mock_parser.rs leakage!
        let mut s3 = accept_until(&listener, &server_stop).expect("turn 3 req");
        let _ = read_request(&mut s3);
        write_sse_tool_call(
            &mut s3,
            "resp-l2",
            "call-l2",
            "(search-choice-program (use rust) (choice resolve (callsite target)) (rule (closed (resolve ?call) (?def)) (ref-at ?call ?name) (same-module-def ?call ?name ?def)) (rule (open (resolve ?call) ?def) (ref-at ?call ?name) (workspace-def ?name ?def)) (rule (closed (resolve ?call) ()) (resolve ?call ?def) (in-test-file ?def)) (solve resolve state-budget: 32 provenance?: #t))",
        );

        let mut s3_cont = accept_until(&listener, &server_stop).expect("turn 3 cont");
        let (_, req3_cont) = read_request(&mut s3_cont);
        let out_l2 = req3_cont["input"].as_array().unwrap().last().unwrap()["output"]
            .as_str()
            .expect("L2 Dusa output");
        assert!(out_l2.contains("mode: choice"), "L2 output: {out_l2}");
        assert!(
            out_l2.contains("worlds: 2 (SAT"),
            "L2 expected 2 worlds: {out_l2}"
        );
        assert!(
            out_l2.contains("[deduction: closed"),
            "L2 missing deduction witness: {out_l2}"
        );
        assert!(
            out_l2.contains("[branch: open"),
            "L2 missing branch witness: {out_l2}"
        );
        assert!(
            !out_l2.contains("mock_parser.rs"),
            "L2 leaked test mock: {out_l2}"
        );
        assert!(
            out_l2.contains("compact_mode: true"),
            "L2 Turn-3 expected compact_mode over gRPC session: {out_l2}"
        );
        eprintln!("\n[PTY LLM TURN 3 — CASE B.1 DUSA PROVENANCE RECEIPT]\n{out_l2}");
        write_sse_final_answer(
            &mut s3_cont,
            "CASE-B-OK: 2 consistent Dusa worlds (parser_a vs parser_b), mock excluded.",
        );
    });

    // Config WITHOUT `[session]` (exact shape of `example.toml`!): with
    // `MARLIN_AGENT_CORE_DEV=1` (provided by `.envrc`), `marlin-tui` must
    // automatically derive `.data/marlin/{session.db,artifacts,search}` from
    // Scheme `DefaultArtifactCatalog.workspace-layout` and bind `tool_catalog`!
    let config = ws.join(".marlin-test-config.toml");
    fs::write(
        &config,
        format!(
            "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://{address}/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n",
        ),
    )
    .unwrap();

    let vars = [
        (
            "MARLIN_ASP_GATEWAY_BIN",
            asp_bin.to_string_lossy().into_owned(),
        ),
        ("MARLIN_AGENT_CORE_DEV", "1".to_owned()),
    ];
    let mut terminal = Terminal::start_in(&config, &[], &vars, 30, 128, ws);
    terminal.wait_status("Message: Enter send");

    // Verify Scheme Artifacts auto-materialized `.data/marlin/` and bound `session.db`!
    assert!(ws.join(".data/marlin/session.db").exists());
    assert!(ws.join(".data/marlin/artifacts").is_dir());
    assert!(ws.join(".data/marlin/search").is_dir());

    // Step 1: Open /helm via SPC s (native Scheme Meow binding!), verify live ASP
    // hydration (`ASP:5p·`), direct 1-keypress `Alt-4` jump straight to
    // `[4:Dusa-Resolve]` (no blind 4x mashing!), and clean rows without `[rust·rg]` noise!
    terminal.enter_normal_mode();
    terminal.send(b" s");
    terminal.wait_for(
        Duration::from_secs(20),
        "Helm with live ASP entities and mode tabs",
        |output| {
            let rows = screen_lines(output);
            rows.iter().any(|l| l.contains("[4:Dusa-Resolve]"))
                && rows.iter().any(|l| l.contains("ASP:5p·"))
        },
    );
    // Direct 1-keypress `Alt-4` (\x1b4) jumps straight to Case B.1 Dusa!
    terminal.send(b"\x1b4");
    terminal.wait_for(Duration::from_secs(15), "HELM-DUSA 2 worlds", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|l| l.contains("HELM-DUSA"))
            && rows.iter().any(|l| l.contains("2 worlds (SAT)"))
            && rows.iter().any(|l| l.contains("W#1·deduction"))
    });
    // Direct 1-keypress `Alt-5` (\x1b5) jumps straight to Case B.2 Dusa Tier (SAT, no budget exhaustion!),
    // then global Emacs `C-a` (\x01) jumps to column 0 of the long S-expression!
    terminal.send(b"\x1b5");
    terminal.wait_for(Duration::from_secs(15), "HELM-DUSA Tier SAT", |output| {
        let rows = screen_lines(output);
        rows.iter().any(|l| l.contains("HELM-DUSA"))
            && rows.iter().any(|l| l.contains("worlds (SAT)"))
            && !rows.iter().any(|l| l.contains("budget-exhausted"))
    });
    terminal.send(b"\x01");
    terminal.wait_for(
        Duration::from_secs(5),
        "Emacs C-a scrolled Helm minibuffer to start",
        |output| {
            screen_lines(output)
                .iter()
                .any(|l| l.contains("Helm> ▏(search-choice"))
        },
    );
    // Close Helm with Ctrl-g, insert residual text, verify Ctrl-g clears minibuffer cleanly!
    terminal.send(b"\x07");
    terminal.wait_for(Duration::from_secs(5), "Helm closed", |output| {
        !screen_lines(output).iter().any(|l| l.contains("Helm>"))
    });
    terminal.send(b"istuck-text");
    terminal.wait_for(Duration::from_secs(5), "composer has text", |output| {
        screen_lines(output)
            .iter()
            .any(|l| l.contains("stuck-text"))
    });
    terminal.send(b"\x07");
    terminal.wait_for(
        Duration::from_secs(5),
        "Ctrl-g cleared minibuffer",
        |output| {
            !screen_lines(output)
                .iter()
                .any(|l| l.contains("stuck-text"))
        },
    );
    thread::sleep(Duration::from_millis(80));

    // Step 2: Run 3 real consecutive LLM turns (L0 Brute-Force -> Case A Ascent -> Case B.1 Dusa)!
    for (prompt, step_name, call_tag, answer_marker) in [
        (
            "Find parse_payload fast\r",
            "L0 brute-search review",
            "[call-l0]",
            "L0-OK: located parse_payload",
        ),
        (
            "List Rust functions via Ascent\r",
            "Case A Ascent review",
            "[call-l1]",
            "CASE-A-OK: enumerated all Rust",
        ),
        (
            "Disambiguate parse_payload with Dusa\r",
            "Case B.1 Dusa review",
            "[call-l2]",
            "CASE-B-OK: 2 consistent Dusa worlds",
        ),
    ] {
        let prompt_visible = prompt.trim();
        terminal.send(prompt.as_bytes());
        terminal.wait_for(Duration::from_secs(20), step_name, |output| {
            let rows = screen_lines(output);
            rows.iter().any(|l| l.contains(prompt_visible))
                && rows.iter().any(|l| l.contains(call_tag))
                && rows.iter().any(|l| l.contains("Approve once"))
        });
        terminal.send(b"\r");
        terminal.wait_for(Duration::from_secs(20), answer_marker, |output| {
            let rows = screen_lines(output);
            rows.iter().any(|l| l.contains(answer_marker))
                && rows.iter().any(|l| l.contains("▸ Tool · workspace_search"))
        });
    }

    // Step 3: Verify structured Tool execution telemetry & OpenTelemetry span appear
    // in the live `Processes` buffer (`M-x list-processes`)!
    terminal.enter_normal_mode();
    terminal.send(b"\x1bx");
    terminal.wait_text("M-x");
    terminal.send(b"list-processes\r");
    terminal.wait_for(
        Duration::from_secs(10),
        "Processes buffer lists OTel tool executions",
        |output| {
            let rows = screen_lines(output);
            rows.iter().any(|l| l.contains("runtime.tool.execute"))
                && rows.iter().any(|l| l.contains("workspace_search"))
                && rows.iter().any(|l| l.contains("Arg:query"))
                && rows.iter().any(|l| l.contains("ASP Semantic Gateway"))
        },
    );

    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(10));
    terminal.assert_restored();
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
}

/// Self-contained Unix PTY end-to-end proof (zero external `asp-gateway` required):
/// launches the real AOT-linked `marlin-tui` binary and executes 4 consecutive
/// LLM tool turns over its loopback HTTP/2 `tonic` gRPC `SearchGatewayService` +
/// SM11 `LinkedSearchDslMailbox`, proving:
/// - Unified `workspace_search` schema advertisement
/// - Turn 1 L0 brute-force + Recency-Peak `dsl_tips:` / `workspace_layers:`
/// - Turn 2 L1 Ascent + automatic `compact_mode: true` unloading
/// - Turn 3 L2 Dusa `(solve-placement ...)` live Scheme `evidence-chain.ss` M1 harvesting
/// - Turn 4 L1 Ascent consuming Turn 3's M1 `verified_tier` facts across turns ("越搜越强")
#[test]
#[ignore = "real PTY AOT marlin-tui 4-turn HTTP/2 gRPC + M1 evidence-chain E2E"]
fn workspace_search_grpc_gateway_progressive_evidence_chain_pty_e2e() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path();
    stage_polyglot_lab_workspace(ws);

    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_on_drop = StopOnDrop(stop.clone());
    let server_stop = stop.clone();

    let server = thread::spawn(move || {
        // Turn 1 (L0 Brute-Force + Recency-Peak Turn-1 self-teaching over HTTP/2 tonic gRPC)
        let mut s1 = accept_until(&listener, &server_stop).expect("turn 1 req");
        let (_, req1) = read_request(&mut s1);
        let tools = req1["tools"].as_array().expect("tools array");
        let search_tool = tools
            .iter()
            .find(|t| t["name"] == "workspace_search")
            .expect("workspace_search tool advertised to LLM");
        let desc = search_tool["description"].as_str().expect("description");
        assert!(desc.contains("(brute-search ...)"), "schema: {desc}");
        assert!(desc.contains("(ascent-fragment ...)"), "schema: {desc}");
        assert!(desc.contains("(search-choice-program ...)"), "schema: {desc}");
        assert!(
            desc.contains("(solve-resolve ") && desc.contains("(find-impact "),
            "schema: {desc}"
        );
        write_sse_tool_call(
            &mut s1,
            "resp-l0",
            "call-l0",
            "(brute-search \"parse_payload\" path-prefix: \"crates/\" lang: \"rust\")",
        );

        let mut s1_cont = accept_until(&listener, &server_stop).expect("turn 1 cont");
        let (_, req1_cont) = read_request(&mut s1_cont);
        let out_l0 = req1_cont["input"].as_array().unwrap().last().unwrap()["output"]
            .as_str()
            .expect("L0 output");
        assert!(out_l0.contains("mode: brute-force"), "{out_l0}");
        assert!(out_l0.contains("app.rs"), "{out_l0}");
        assert!(
            out_l0.contains("[evidence_frontier |")
                && out_l0.contains("teaching_kit: active")
                && out_l0.contains("dsl_tips:")
                && out_l0.contains("workspace_layers:"),
            "Turn 1 missing gRPC Recency-Peak self-teaching frontier: {out_l0}"
        );
        write_sse_final_answer(&mut s1_cont, "PTY-GRPC-T1-OK");

        // Turn 2 (L1 Ascent via SM11 LinkedSearchDslMailbox + Turn-2 compact_mode auto-unloading)
        let mut s2 = accept_until(&listener, &server_stop).expect("turn 2 req");
        let _ = read_request(&mut s2);
        write_sse_tool_call(
            &mut s2,
            "resp-l1",
            "call-l1",
            "(ascent-fragment ((answer path name line) <-- (file f \"rust\" path) (rust_fn _ f name line)))",
        );

        let mut s2_cont = accept_until(&listener, &server_stop).expect("turn 2 cont");
        let (_, req2_cont) = read_request(&mut s2_cont);
        let out_l1 = req2_cont["input"].as_array().unwrap().last().unwrap()["output"]
            .as_str()
            .expect("L1 output");
        assert!(out_l1.contains("mode: ascent"), "{out_l1}");
        assert!(out_l1.contains("local_init"), "{out_l1}");
        assert!(out_l1.contains("parse_payload"), "{out_l1}");
        assert!(
            out_l1.contains("compact_mode: true") && !out_l1.contains("dsl_tips:"),
            "Turn 2 expected compact_mode auto-unloading: {out_l1}"
        );
        write_sse_final_answer(&mut s2_cont, "PTY-GRPC-T2-OK");

        // Turn 3 (L2 Dusa solve-placement harvesting M1 `verified_tier` in TUI's AOT Scheme VM!)
        let mut s3 = accept_until(&listener, &server_stop).expect("turn 3 req");
        let _ = read_request(&mut s3);
        write_sse_tool_call(
            &mut s3,
            "resp-l2",
            "call-l2",
            "(solve-placement path-prefix: \"scheme/search-dsl/\" module-limit: auto choices: (resident provider) forbid-hot-path: provider state-budget: 64)",
        );

        let mut s3_cont = accept_until(&listener, &server_stop).expect("turn 3 cont");
        let (_, req3_cont) = read_request(&mut s3_cont);
        let out_l2 = req3_cont["input"].as_array().unwrap().last().unwrap()["output"]
            .as_str()
            .expect("L2 output");
        assert!(out_l2.contains("mode: choice"), "{out_l2}");
        assert!(out_l2.contains("compact_mode: true"), "{out_l2}");
        assert!(
            out_l2.contains("(+1)") && out_l2.contains("status: unique-deduction"),
            "{out_l2}"
        );
        write_sse_final_answer(&mut s3_cont, "PTY-GRPC-T3-OK");

        // Turn 4 (L1 Ascent directly consuming Turn 3's M1 `verified_tier` over the TUI session!)
        let mut s4 = accept_until(&listener, &server_stop).expect("turn 4 req");
        let _ = read_request(&mut s4);
        write_sse_tool_call(
            &mut s4,
            "resp-m1",
            "call-m1",
            "(ascent-fragment ((answer m lvl) <-- (verified_tier m lvl)))",
        );

        let mut s4_cont = accept_until(&listener, &server_stop).expect("turn 4 cont");
        let (_, req4_cont) = read_request(&mut s4_cont);
        let out_m1 = req4_cont["input"].as_array().unwrap().last().unwrap()["output"]
            .as_str()
            .expect("M1 output");
        assert!(out_m1.contains("mode: ascent"), "{out_m1}");
        assert!(
            out_m1.contains("funcs.ss") && out_m1.contains("resident"),
            "Turn 4 expected M1 verified_tier facts harvested on Turn 3: {out_m1}"
        );
        assert!(out_m1.contains("compact_mode: true"), "{out_m1}");
        write_sse_final_answer(&mut s4_cont, "PTY-GRPC-T4-M1-OK");
    });

    let config = ws.join(".marlin-test-config.toml");
    fs::write(
        &config,
        format!(
            "provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://{address}/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n",
        ),
    )
    .unwrap();

    let vars = [("MARLIN_AGENT_CORE_DEV", "1".to_owned())];
    let mut terminal = Terminal::start_in(&config, &[], &vars, 30, 128, ws);
    terminal.wait_status("Message: Enter send");

    assert!(ws.join(".data/marlin/session.db").exists());

    for (prompt, step_name, call_tag, answer_marker) in [
        ("Turn 1 L0\r", "L0 review", "[call-l0]", "PTY-GRPC-T1-OK"),
        ("Turn 2 L1\r", "L1 review", "[call-l1]", "PTY-GRPC-T2-OK"),
        ("Turn 3 L2\r", "L2 review", "[call-l2]", "PTY-GRPC-T3-OK"),
        ("Turn 4 M1\r", "M1 review", "[call-m1]", "PTY-GRPC-T4-M1-OK"),
    ] {
        let prompt_visible = prompt.trim();
        terminal.send(prompt.as_bytes());
        terminal.wait_for(Duration::from_secs(20), step_name, |output| {
            let rows = screen_lines(output);
            rows.iter().any(|l| l.contains(prompt_visible))
                && rows.iter().any(|l| l.contains(call_tag))
                && rows.iter().any(|l| l.contains("Approve once"))
        });
        terminal.send(b"\r");
        terminal.wait_for(Duration::from_secs(20), answer_marker, |output| {
            let rows = screen_lines(output);
            rows.iter().any(|l| l.contains(answer_marker))
                && rows.iter().any(|l| l.contains("▸ Tool · workspace_search"))
        });
    }

    terminal.enter_normal_mode();
    terminal.send(b"\x1bx");
    terminal.wait_text("M-x");
    terminal.send(b"list-processes\r");
    terminal.wait_for(
        Duration::from_secs(10),
        "Processes buffer lists OTel tool executions",
        |output| {
            let rows = screen_lines(output);
            rows.iter().any(|l| l.contains("runtime.tool.execute"))
                && rows.iter().any(|l| l.contains("workspace_search"))
                && rows.iter().any(|l| l.contains("Arg:query"))
        },
    );

    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(10));
    terminal.assert_restored();
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
}
