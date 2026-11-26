// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Dynamic embedded-VM Marlin Scheme Authority Host daemon (`marlin-scheme-host`).

#[cfg(not(feature = "external-program"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    marlin_gerbil_scheme::scheme_host_runtime::run_dynamic_scheme_host()
}

#[cfg(feature = "external-program")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    Err("marlin-scheme-host requires default dynamic VM mode (without `external-program`); use `marlin-scheme-host-aot` for Linked-AOT mode".into())
}
