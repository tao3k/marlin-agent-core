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

#[test]
fn discovers_gambit_library_from_gerbil_shell_wrapper() {
    let prefix = unique_temp_dir("marlin-gerbil-native-build-wrapper-prefix");
    let wrapper_root = unique_temp_dir("marlin-gerbil-native-build-wrapper-bin");
    let wrapper_bin = wrapper_root.join("bin");
    let real_bin = prefix.join("bin");
    let lib = prefix.join("lib");
    fs::create_dir_all(&wrapper_bin).expect("create fake wrapper bin");
    fs::create_dir_all(&real_bin).expect("create fake real bin");
    fs::create_dir_all(&lib).expect("create fake real lib");
    fs::write(real_bin.join("gsc"), "").expect("write fake real gsc");
    fs::write(lib.join("libgambit.a"), "").expect("write fake gambit lib");
    fs::write(
        wrapper_bin.join("gsc"),
        format!(
            "#!/bin/sh\nexport GERBIL_HOME=\"{}\"\nexec \"{}\" \"$@\"\n",
            prefix.display(),
            real_bin.join("gsc").display()
        ),
    )
    .expect("write fake gsc wrapper");

    let discovery = find_gambit_link_search_dir_from_gsc(&wrapper_bin.join("gsc"))
        .expect("discover gambit lib from Gerbil shell wrapper");

    assert_eq!(discovery.search_dir, lib);
    assert!(discovery.library_path.ends_with("libgambit.a"));

    fs::remove_dir_all(prefix).expect("remove fake prefix");
    fs::remove_dir_all(wrapper_root).expect("remove fake wrapper root");
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}
