// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
#![cfg(feature = "org-source")]

#[path = "org_store/discovery.rs"]
mod discovery;
#[path = "org_store/file.rs"]
mod file;
#[path = "org_store/memory.rs"]
mod memory;
