// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! ASP Rust coding-style and module-ownership gate for `marlin-agent-app-server`.
//! Fail-closed on every `Error` and `Warning` finding (`0 Error`, `0 Warning`) across
//! all library modules, thin binary launchers, and separated unit/integration test suites.

asp_rust::asp_rust_cargo_test_gate!(
    advice = allow,
    config = {
        asp_rust::default_asp_rust_config().with_cargo_test_advice_allow_explanation(
            "scope=marlin-agent-app-server non-blocking Info advice only (Error and Warning remain fail-closed at 0); owner=app-server-maintainers; finding_category=docs-and-control-flow-advisory; why_safe_now=all blocking Error and Warning findings are zero and SM26-SM28 security and runtime invariants are enforced by dedicated unit and live gRPC integration tests; cleanup_trigger=promote to strict mode=deny once remaining public-doc and control-flow Info advisories are retired",
        )
    }
);
