use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_gerbil_native_build::write_native_c_header;

#[test]
fn cbindgen_generates_minimal_c_header() {
    let root = unique_temp_dir("marlin-gerbil-native-build-cbindgen");
    let src = root.join("src");
    fs::create_dir_all(&src).expect("create temp crate src");
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "marlin-gerbil-native-cbindgen-smoke"
version = "0.0.0"
edition = "2024"

[lib]
path = "src/lib.rs"
"#,
    )
    .expect("write temp Cargo.toml");
    fs::write(
        src.join("lib.rs"),
        r#"#[repr(C)]
pub struct MarlinDeckRuntimeUtf8 {
    pub ptr: *const u8,
    pub len: usize,
}

#[unsafe(no_mangle)]
pub extern "C" fn marlin_deck_runtime_accept_utf8(input: MarlinDeckRuntimeUtf8) -> usize {
    input.len
}
"#,
    )
    .expect("write temp lib.rs");

    let header = root.join("target/marlin_deck_runtime_native.h");
    let receipt = write_native_c_header(&root, &header, "MARLIN_DECK_RUNTIME_NATIVE_GENERATED_H")
        .expect("generate cbindgen header");
    let content = fs::read_to_string(&receipt.header_file).expect("read generated header");

    assert!(content.contains("MARLIN_DECK_RUNTIME_NATIVE_GENERATED_H"));
    assert!(content.contains("MarlinDeckRuntimeUtf8"));
    assert!(content.contains("marlin_deck_runtime_accept_utf8"));

    fs::remove_dir_all(root).expect("remove temp crate");
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}
