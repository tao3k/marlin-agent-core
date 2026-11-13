// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

mod projection;

pub use projection::{
    ToolingProjectionError, catalog_from_declarations, decode_tooling, project_linked,
};
