#[path = "../../src/discovery.rs"]
mod implementation;

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use implementation::find_gambit_link_search_dir_from_gsc;

#[test]
fn finds_gambit_library_from_gsc_prefix_lib() {
    let root = unique_temp_dir("marlin-gerbil-native-build-gambit-prefix");
    let bin = root.join("bin");
    let lib = root.join("lib");
    fs::create_dir_all(&bin).expect("create fake bin");
    fs::create_dir_all(&lib).expect("create fake lib");
    fs::write(bin.join("gsc"), "").expect("write fake gsc");
    fs::write(lib.join("libgambit.a"), "").expect("write fake gambit lib");

    let discovery = find_gambit_link_search_dir_from_gsc(&bin.join("gsc"))
        .expect("discover gambit lib from gsc prefix");

    assert_eq!(discovery.search_dir, lib);
    assert!(discovery.library_path.ends_with("libgambit.a"));

    fs::remove_dir_all(root).expect("remove fake prefix");
}

#[test]
fn discovers_versioned_linux_shared_library() {
    let root = unique_temp_dir("marlin-gerbil-native-build-gambit-so");
    let bin = root.join("bin");
    let lib = root.join("lib");
    fs::create_dir_all(&bin).expect("create fake bin");
    fs::create_dir_all(&lib).expect("create fake lib");
    fs::write(bin.join("gsc"), "").expect("write fake gsc");
    fs::write(lib.join("libgambit.so.4.9.7"), "").expect("write fake gambit so");

    let discovery = find_gambit_link_search_dir_from_gsc(&bin.join("gsc"))
        .expect("discover versioned gambit shared library");

    assert_eq!(discovery.search_dir, lib);
    assert!(discovery.library_path.ends_with("libgambit.so.4.9.7"));

    fs::remove_dir_all(root).expect("remove fake prefix");
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}
