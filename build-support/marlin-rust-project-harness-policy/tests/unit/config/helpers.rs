use std::path::{Path, PathBuf};

use rust_lang_project_harness::{
    RustOwnerResponsibility, RustVerificationProfileHint, RustVerificationTaskKind,
};

pub(crate) fn profile_hint<'a>(
    hints: &'a [RustVerificationProfileHint],
    owner_path: &str,
) -> &'a RustVerificationProfileHint {
    hints
        .iter()
        .find(|hint| hint.owner_path == Path::new(owner_path))
        .unwrap_or_else(|| panic!("missing profile hint for {owner_path}"))
}

pub(crate) fn assert_responsibilities<const N: usize>(
    hint: &RustVerificationProfileHint,
    responsibilities: [RustOwnerResponsibility; N],
) {
    for responsibility in responsibilities {
        assert!(
            hint.responsibilities.contains(&responsibility),
            "missing responsibility {responsibility:?} in {:?}",
            hint.responsibilities
        );
    }
}

pub(crate) fn assert_performance_and_stability_tasks(hint: &RustVerificationProfileHint) {
    let task_kinds = hint.task_kinds.as_ref().expect("explicit task kinds");
    assert!(task_kinds.contains(&RustVerificationTaskKind::Performance));
    assert!(task_kinds.contains(&RustVerificationTaskKind::Stability));
}

pub(crate) fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("policy crate should live under workspace/build-support")
        .to_path_buf()
}
