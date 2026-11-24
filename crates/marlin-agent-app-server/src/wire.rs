// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Generated Rust Protobuf & Tonic bindings for the complete 7-package
//! `marlin.*.v1` wire family shared with `tao3k/marlin-mygo`.

#![allow(clippy::all)]

pub mod marlin {
    pub mod common {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/ui_wire/marlin.common.v1.rs"));
        }
    }
    pub mod app {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/ui_wire/marlin.app.v1.rs"));
        }
    }
    pub mod ui {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/ui_wire/marlin.ui.v1.rs"));
        }
    }
    pub mod gate {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/ui_wire/marlin.gate.v1.rs"));
        }
    }
    pub mod plugin {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/ui_wire/marlin.plugin.v1.rs"));
        }
    }
    pub mod device {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/ui_wire/marlin.device.v1.rs"));
        }
    }
    pub mod projection {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/ui_wire/marlin.projection.v1.rs"));
        }
    }
}

/// Compiled canonical `FileDescriptorSet` covering all 7 `marlin.*.v1` packages.
pub const UI_WIRE_FILE_DESCRIPTOR_SET: &[u8] =
    tonic::include_file_descriptor_set!("marlin-ui-wire-v1");
