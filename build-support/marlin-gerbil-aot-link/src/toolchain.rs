//! C compiler discovery from the Gerbil bridge.

pub use gerbil_scheme_aot_build::{
    NativeCCompilerTool as CCompilerTool, discover_native_c_compiler as discover_c_compiler,
};
