// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "grpc")]
    {
        const SCHEMA: &str = "proto/marlin/search/v1/search_step.proto";
        println!("cargo:rerun-if-changed={SCHEMA}");
        tonic_prost_build::configure()
            .build_server(true)
            .build_client(true)
            .compile_protos(&[SCHEMA], &["proto"])?;
    }
    Ok(())
}
