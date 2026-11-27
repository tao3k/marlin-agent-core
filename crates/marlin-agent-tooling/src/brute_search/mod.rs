// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! L0 pure-Rust parallel resident RAM fast-path forms (zero Gerbil VM lock, <1ms):
//! - `(brute-search "pat" path-prefix: "..." exclude-path-prefix: "tests/" lang: "rust" context: 2 limit: 32)`
//! - `(file-outline "path-or-dir/" lang: "rust" limit: 80)`
//! - `(read-file "path/to/file.rs" start: 1 end: 120)`

mod execute;
mod outline_read;
mod parser;

pub(crate) use execute::execute_l0_search;
pub(crate) use parser::is_l0_search_form;
