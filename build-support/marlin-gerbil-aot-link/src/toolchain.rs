// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! C compiler discovery from the Gerbil bridge.

pub use gerbil_scheme_aot_build::{
    NativeCCompilerTool as CCompilerTool, discover_native_c_compiler as discover_c_compiler,
};
