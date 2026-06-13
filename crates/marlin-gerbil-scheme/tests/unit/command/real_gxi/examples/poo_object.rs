use super::{local_gxi, support::test_root};
use marlin_gerbil_scheme::{
    GERBIL_LOADPATH_ENV, GERBIL_POO_OBJECT_MODULE, GERBIL_POO_PACKAGE_NAME,
    gerbil_runtime_loadpath, write_gerbil_runtime_assets,
};
use std::{fs, path::Path, process::Command};

#[test]
#[ignore = "requires a local Gerbil gxi executable and installed gerbil-poo dependency"]
fn command_compiler_real_gxi_deck_runtime_can_execute_poo_object_probe_when_dependency_installed() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    if !poo_object_module_available(&gxi) {
        eprintln!(
            "skipping POO object probe because {GERBIL_POO_OBJECT_MODULE} from {GERBIL_POO_PACKAGE_NAME} is not installed"
        );
        return;
    }

    let root = test_root("deck-runtime-poo-object");
    write_gerbil_runtime_assets(root.path()).expect("write gerbil runtime assets");
    let probe = root.path().join("deck-runtime-poo-object.ss");
    write_poo_object_probe(&probe);

    let output = Command::new(gxi)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(root.path()))
        .arg(probe)
        .output()
        .expect("run real gxi deck runtime POO object probe");

    assert!(
        output.status.success(),
        "gxi deck runtime POO object probe failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let response: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("decode deck runtime POO object probe");
    assert_eq!(response["package"], "marlin-deck-runtime");
    assert_eq!(response["module"], ":marlin/deck-runtime");
    assert_eq!(response["object_system"], GERBIL_POO_PACKAGE_NAME);
}

fn poo_object_module_available(gxi: &Path) -> bool {
    Command::new(gxi)
        .arg("-e")
        .arg(format!("(import {GERBIL_POO_OBJECT_MODULE})"))
        .arg("-e")
        .arg("(display \"ok\")")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn write_poo_object_probe(path: &Path) {
    fs::write(
        path,
        r#"(import :marlin/deck-runtime
        :clan/poo/object)

(def deck-runtime
  (.o package: marlin-deck-runtime-package-name
      module: marlin-deck-runtime-module
      object-system: marlin-deck-runtime-poo-package-name))

(display "{\"package\":\"")
(display (.get deck-runtime package))
(display "\",\"module\":\"")
(display (.get deck-runtime module))
(display "\",\"object_system\":\"")
(display (.get deck-runtime object-system))
(display "\"}")
(newline)
"#,
    )
    .expect("write deck runtime POO object probe");
}
