// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "client")]
    {
        const SCHEMA: &str = "proto/marlin/app/v1/app_server.proto";
        println!("cargo:rerun-if-changed={SCHEMA}");
        let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output"));
        tonic_prost_build::configure()
            .build_server(cfg!(feature = "grpc"))
            .file_descriptor_set_path(out.join("marlin-app-v1.bin"))
            .compile_protos(&[SCHEMA], &["proto"])?;
    }

    #[cfg(feature = "ui-grpc")]
    {
        const UI_SCHEMAS: &[&str] = &[
            "proto-ui/marlin/common/v1/common.proto",
            "proto-ui/marlin/app/v1/app_server.proto",
            "proto-ui/marlin/ui/v1/ui.proto",
            "proto-ui/marlin/gate/v1/gate.proto",
            "proto-ui/marlin/plugin/v1/plugin.proto",
            "proto-ui/marlin/device/v1/device.proto",
            "proto-ui/marlin/projection/v1/projection.proto",
        ];
        for schema in UI_SCHEMAS {
            println!("cargo:rerun-if-changed={schema}");
        }
        let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output"));
        let ui_out = out.join("ui_wire");
        std::fs::create_dir_all(&ui_out)?;
        tonic_prost_build::configure()
            .build_server(true)
            .build_client(true)
            .out_dir(&ui_out)
            .file_descriptor_set_path(out.join("marlin-ui-wire-v1.bin"))
            .compile_protos(UI_SCHEMAS, &["proto-ui"])?;
    }

    Ok(())
}
