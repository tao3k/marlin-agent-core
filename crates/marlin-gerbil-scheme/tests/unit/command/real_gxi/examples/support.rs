pub(super) use crate::command::real_gxi::{MARLIN_REQUIRE_REAL_GXI_ENV, local_gxi};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::{Builder, TempDir};

pub(super) fn test_root(name: &str) -> TempDir {
    Builder::new()
        .prefix(&format!("marlin-gerbil-scheme-{name}-"))
        .tempdir()
        .unwrap_or_else(|error| panic!("creates {name} test root: {error}"))
}

pub(super) fn local_gxtest_for_gxi(gxi: &Path) -> Option<PathBuf> {
    let Some(parent) = gxi.parent() else {
        return missing_gxtest(gxi.with_file_name("gxtest"));
    };
    let gxtest = parent.join("gxtest");
    if gxtest.exists() {
        return Some(gxtest);
    }
    missing_gxtest(gxtest)
}

fn missing_gxtest(gxtest: PathBuf) -> Option<PathBuf> {
    let message = format!(
        "skipping real gxtest because {} is missing",
        gxtest.display()
    );
    if std::env::var_os(MARLIN_REQUIRE_REAL_GXI_ENV).is_some() {
        panic!("{message}; unset {MARLIN_REQUIRE_REAL_GXI_ENV} or install matching gxtest");
    }
    eprintln!("{message}");
    None
}

pub(super) fn write_protocol_bindings_example(example: &Path) {
    fs::write(
        example,
        r#"(import :marlin/protocol)

(def patch
  (make-marlin-workspace-patch
   "gerbil intent"
   "gerbil"
   (list (make-marlin-set-todo-op "memory.org:1:goal" "Done")
         (make-marlin-set-property-op "memory.org:1:goal" "OWNER" "gerbil")
         (make-marlin-mark-memory-candidate-op "memory.org:1:goal" "long-term"))))

(def intent
  (make-marlin-workspace-patch-intent "intent:memory" patch #t))

(def artifact
  (make-marlin-workspace-patch-intent-artifact intent))

(display "workspace-patch-intent-artifact")
(newline)
"#,
    )
    .expect("write protocol bindings example");
}

pub(super) fn write_deck_runtime_handshake_example(example: &Path) {
    fs::write(
        example,
        r#"(import :marlin/deck-runtime)

(write (marlin-deck-runtime-capability-fact))
(newline)
"#,
    )
    .expect("write deck runtime handshake example");
}
