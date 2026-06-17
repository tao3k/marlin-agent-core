use marlin_gerbil_native_build::discover_native_c_compiler;

#[test]
fn discovers_native_c_compiler_when_toolchain_is_available() {
    let Ok(tool) = discover_native_c_compiler() else {
        return;
    };

    assert!(!tool.program.as_os_str().is_empty());
}
