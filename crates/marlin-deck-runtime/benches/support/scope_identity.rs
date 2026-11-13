// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Independent known answers for the fixed benchmark fixture only.
//! This is not a runtime Context encoder or an admission implementation.

use sha2::{Digest, Sha256};

pub fn expected_digest(index: usize) -> [u8; 32] {
    // The fixture contains no quotes or escapes. These are the canonical UTF-8
    // bytes of Lab's scope-v1 tuple, with its Unicode task and session turn.
    let tuple = format!(
        "(ai-agentic-context/scope-v1 \"marlin\" \"example\" 1 \"project\" \"worktree\" \"benchmark-source\" \"assistant\" \"任务-{index}\" \"panel\" \"conversation\" {index})"
    );
    Sha256::digest(tuple.as_bytes()).into()
}

pub fn verify<'a>(
    expected: &[[u8; 32]],
    observed: impl ExactSizeIterator<Item = (usize, &'a [u8; 32])>,
) -> Result<(), &'static str> {
    if observed.len() != expected.len() {
        return Err("Context receipt count differs from the workload");
    }
    let mut seen = vec![false; expected.len()];
    for (index, digest) in observed {
        let Some(answer) = expected.get(index) else {
            return Err("Context receipt index is outside the workload");
        };
        if seen[index] {
            return Err("Context receipt index was repeated");
        }
        if digest != answer {
            return Err("Context digest differs from the independent known answer");
        }
        seen[index] = true;
    }
    if seen.iter().any(|seen| !seen) {
        return Err("Context receipt is missing");
    }
    Ok(())
}
