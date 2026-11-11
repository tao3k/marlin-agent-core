// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::path::Path;

use asp_rust::{RustOwnerResponsibility, RustVerificationProfileHint, RustVerificationTaskKind};

pub(crate) use crate::workspace::workspace_root;

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
