// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Resource limits for admitting a workbench projection.

// Resource ceiling for the workbench, independent of Scheme's Buffer catalog.
pub(crate) const MAX_REGIONS: usize = 64;
pub(crate) const MAX_HOST_STATUS: usize = 8;
// Five visible slots, ten runtime/transport observations and eight host contributions.
pub(crate) const MODELINE_SLOTS: usize = 15;
pub(crate) const MAX_STATUS: usize = 23;
pub(crate) const MAX_EXTENSIONS: usize = 16;
pub(crate) const MAX_METHOD_FACTORIES: usize = 16;
pub(crate) const MAX_COMMANDS: usize = 256;
pub(crate) const MAX_TEXT_CHARS: usize = 256;
pub(crate) const MAX_ID_BYTES: usize = 1024;

// Keep host contributions inside the bounded projection envelope.
const _: () = assert!(MAX_STATUS == MODELINE_SLOTS + MAX_HOST_STATUS);
const _: () = assert!(MAX_REGIONS > 3 + MAX_EXTENSIONS + 1);
