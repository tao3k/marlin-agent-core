pub(super) use crate::command::real_gxi::{
    MARLIN_REQUIRE_REAL_GXI_ENV, assert_workspace_patch_intent_artifact, local_gxi,
};
use std::{fs, path::Path};
use tempfile::{Builder, TempDir};

pub(super) fn test_root(name: &str) -> TempDir {
    Builder::new()
        .prefix(&format!("marlin-gerbil-scheme-{name}-"))
        .tempdir()
        .unwrap_or_else(|error| panic!("creates {name} test root: {error}"))
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

(display-marlin-compile-response
 (make-marlin-workspace-patch-intent-artifact intent))
(newline)
"#,
    )
    .expect("write protocol bindings example");
}

pub(super) fn write_deck_runtime_handshake_example(example: &Path) {
    fs::write(
        example,
        r#"(import :marlin/deck-runtime)

(display-marlin-deck-runtime-capability-json)
(newline)
"#,
    )
    .expect("write deck runtime handshake example");
}
