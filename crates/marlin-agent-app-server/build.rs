// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "grpc")]
    {
        const SCHEMA: &str = "proto/marlin/app/v1/app_server.proto";
        println!("cargo:rerun-if-changed={SCHEMA}");
        tonic_prost_build::compile_protos(SCHEMA)?;
    }
    Ok(())
}
