// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Pure lookup helpers over Scheme-projected [`HelmSearchPresetSpec`] entries.
//!
//! Preset definitions live strictly in Scheme (`scheme/ui-helm/objects.ss`
//! `HelmSearchPresets.`), projected into Rust over the zero-JSON C-ABI — zero
//! hardcoded preset queries or language tables in Rust.

use marlin_agent_search_engine::HelmSearchPresetSpec;

/// Resolve a shorthand `M-x helm <alias>` argument (e.g. `dusa`, `ascent`, `tier`)
/// against the Scheme-projected preset catalog.
pub(crate) fn preset_index_by_alias(presets: &[HelmSearchPresetSpec], raw: &str) -> Option<usize> {
    let key = raw.trim().to_ascii_lowercase();
    if key.is_empty() {
        return None;
    }
    presets
        .iter()
        .position(|p| p.alias.eq_ignore_ascii_case(&key) || p.shortcut.eq_ignore_ascii_case(&key))
}
