//! Native C toolchain discovery for Gerbil AOT build scripts.

use std::path::PathBuf;

/// C compiler selected through the standard `cc` crate environment contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeCCompilerTool {
    pub program: PathBuf,
}

/// Discovers the platform C compiler without compiling any source.
pub fn discover_native_c_compiler() -> Result<NativeCCompilerTool, String> {
    cc::Build::new()
        .try_get_compiler()
        .map(|tool| NativeCCompilerTool {
            program: tool.path().to_path_buf(),
        })
        .map_err(|error| error.to_string())
}
