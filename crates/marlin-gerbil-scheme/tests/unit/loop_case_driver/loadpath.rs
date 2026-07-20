use std::path::Path;

use marlin_gerbil_scheme::gerbil_config_interface_loadpath_with_src;

#[test]
fn config_interface_loadpath_includes_package_sources_and_local_dependencies() {
    let gerbil_root = Path::new("fixture-gerbil-root");
    let paths = std::env::split_paths(&gerbil_config_interface_loadpath_with_src(gerbil_root))
        .collect::<Vec<_>>();

    assert_eq!(paths[0], gerbil_root.join("src"));
    assert_eq!(paths[1], gerbil_root.join(".gerbil").join("lib"));
}
