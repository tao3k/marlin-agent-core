// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Structured Dusa POPL 2025 multi-world provenance decoder (`choice-report`
//! and `choice-budget-exhausted`) for Marlin's zero-JSON Scheme Search DSL.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use agent_semantic_search::WorkspaceEdbSnapshot;
use gerbil_scheme::{CollectionKind, ObjectKind, RootedSchemeObject, SchemeScalar};

use super::codec::{
    decode_string, decode_symbol, decode_usize, format_scalar_cell, infer_path_and_line, native,
};
use super::render::format_snapshot_header;
use super::types::SearchDslDecodeContext;
use super::{
    SearchDslChoiceRuleKind, SearchDslChoiceWitness, SearchDslChoiceWitnessKind,
    SearchDslChoiceWorld, SearchDslExecutionMode, SearchDslStructuredResult,
    SearchDslStructuredRow,
};
use crate::SearchPlanProjectionError;

const MAX_BUDGET_MSG_BYTES: usize = 4096;

fn decode_cell_vector(
    value: RootedSchemeObject<'_>,
) -> Result<Vec<String>, SearchPlanProjectionError> {
    if native(value.as_ref().kind())? != ObjectKind::Vector {
        return Ok(vec![format_scalar_cell(value)?]);
    }
    let items = native(value.into_collection(CollectionKind::Vector))?;
    let mut out = Vec::with_capacity(items.len());
    for i in 0..items.len() {
        out.push(format_scalar_cell(native(items.element(i))?)?);
    }
    Ok(out)
}

fn decode_witness(
    value: RootedSchemeObject<'_>,
) -> Result<SearchDslChoiceWitness, SearchPlanProjectionError> {
    let tuple = native(value.into_collection(CollectionKind::Vector))?;
    if tuple.len() != 5 {
        return Err(SearchPlanProjectionError::Invalid(
            "Dusa provenance witness must be a 5-vector",
        ));
    }
    let kind_raw = decode_symbol(native(tuple.element(0))?)?;
    let fact = decode_cell_vector(native(tuple.element(1))?)?;
    let rule_kind_raw = format_scalar_cell(native(tuple.element(2))?)?;
    let premises = decode_cell_vector(native(tuple.element(3))?)?;
    let alternatives = decode_cell_vector(native(tuple.element(4))?)?;
    Ok(SearchDslChoiceWitness {
        kind: SearchDslChoiceWitnessKind::parse(&kind_raw),
        fact,
        rule_kind: SearchDslChoiceRuleKind::parse(&rule_kind_raw),
        premises,
        alternatives,
    })
}

fn decode_choice_world(
    value: RootedSchemeObject<'_>,
) -> Result<SearchDslChoiceWorld, SearchPlanProjectionError> {
    let tuple = native(value.into_collection(CollectionKind::Vector))?;
    if tuple.len() != 4 {
        return Err(SearchPlanProjectionError::Invalid(
            "Dusa choice-world must be a 4-vector",
        ));
    }
    let _tag = decode_symbol(native(tuple.element(0))?)?;
    let world_index = decode_usize(native(tuple.element(1))?)?;
    let bindings_vec = native(native(tuple.element(2))?.into_collection(CollectionKind::Vector))?;
    let mut bindings = Vec::with_capacity(bindings_vec.len());
    for i in 0..bindings_vec.len() {
        bindings.push(decode_cell_vector(native(bindings_vec.element(i))?)?);
    }
    let witnesses_vec = native(native(tuple.element(3))?.into_collection(CollectionKind::Vector))?;
    let mut witnesses = Vec::with_capacity(witnesses_vec.len());
    for i in 0..witnesses_vec.len() {
        witnesses.push(decode_witness(native(witnesses_vec.element(i))?)?);
    }
    Ok(SearchDslChoiceWorld {
        world_index,
        bindings,
        witnesses,
    })
}

fn format_witness_tag(binding: &[String], witnesses: &[SearchDslChoiceWitness]) -> String {
    let matched = witnesses
        .iter()
        .find(|w| w.fact.len() == binding.len() + 1 && w.fact.iter().skip(1).eq(binding.iter()));
    let Some(w) = matched else {
        return String::new();
    };
    let mut tag = format!("  [{}: {}", w.kind.as_str(), w.rule_kind.as_str());
    if !w.alternatives.is_empty() && w.kind == SearchDslChoiceWitnessKind::Branch {
        let _ = write!(tag, " | alts: {}", w.alternatives.join(", "));
    }
    if !w.premises.is_empty() {
        let _ = write!(tag, " | via: {}", w.premises[0]);
    }
    tag.push(']');
    tag
}

fn summarize_forced_and_free_bindings(worlds: &[SearchDslChoiceWorld]) -> Option<String> {
    if worlds.is_empty() {
        return None;
    }
    let mut values_by_key: std::collections::BTreeMap<&str, std::collections::BTreeSet<&str>> =
        std::collections::BTreeMap::new();
    worlds
        .iter()
        .flat_map(|w| w.bindings.iter())
        .filter(|b| b.len() == 2)
        .for_each(|b| {
            values_by_key
                .entry(b[0].as_str())
                .or_default()
                .insert(b[1].as_str());
        });
    if values_by_key.is_empty() {
        return None;
    }
    let forced: Vec<String> = values_by_key
        .iter()
        .filter(|(_, vals)| vals.len() == 1)
        .map(|(k, vals)| format!("{k}={}", vals.iter().next().copied().unwrap_or("")))
        .collect();
    let free: Vec<String> = values_by_key
        .iter()
        .filter(|(_, vals)| vals.len() > 1)
        .map(|(k, vals)| {
            let joined = vals.iter().copied().collect::<Vec<_>>().join("|");
            format!("{k}={{{joined}}}")
        })
        .collect();
    Some(format!(
        "forced({}): {} | free({}): {}",
        forced.len(),
        if forced.is_empty() {
            "none".to_owned()
        } else {
            forced.join(", ")
        },
        free.len(),
        if free.is_empty() {
            "none".to_owned()
        } else {
            free.join(", ")
        },
    ))
}

fn append_world_rows(
    world: &SearchDslChoiceWorld,
    known_paths: &HashSet<&str>,
    file_id_map: &HashMap<usize, &str>,
    output_limit: usize,
    rendered_text: &mut String,
    rows: &mut Vec<SearchDslStructuredRow>,
) {
    let header = format!("World #{}:\n", world.world_index);
    if rendered_text.len() + header.len() <= output_limit {
        rendered_text.push_str(&header);
    }
    for binding in &world.bindings {
        let binding_str = if binding.len() == 2 {
            format!("{} -> {}", binding[0], binding[1])
        } else {
            format!("({})", binding.join(" "))
        };
        let witness_tag = format_witness_tag(binding, &world.witnesses);
        let display = format!("[W#{}] {binding_str}{witness_tag}", world.world_index);
        let line_str = format!("  - {binding_str}{witness_tag}\n");
        if rendered_text.len() + line_str.len() <= output_limit {
            rendered_text.push_str(&line_str);
        }
        let (path, line) = infer_path_and_line(binding, known_paths, file_id_map);
        rows.push(SearchDslStructuredRow {
            cells: binding.clone(),
            display,
            path,
            line,
        });
    }
}

/// Decode a rooted Dusa `choice-report` or `choice-budget-exhausted` vector into
/// a structured [`SearchDslStructuredResult`] with full multi-world provenance,
/// combinatorics diagnostics, and partial SAT witness samples when budget-limited.
pub(super) fn decode_choice_payload(
    relation: String,
    payload_obj: RootedSchemeObject<'_>,
    repairs: Vec<String>,
    canonical_query: Option<String>,
    snapshot: &WorkspaceEdbSnapshot,
    ctx: SearchDslDecodeContext<'_>,
) -> Result<SearchDslStructuredResult, SearchPlanProjectionError> {
    let payload = native(payload_obj.into_collection(CollectionKind::Vector))?;
    let tag = if payload.is_empty() {
        String::new()
    } else {
        decode_symbol(native(payload.element(0))?)?
    };

    let mut rendered_text = format_snapshot_header(snapshot, ctx.prefilter);
    let _ = writeln!(
        rendered_text,
        "mode: choice (Dusa POPL 2025 finite-choice solver)\nrelation: {relation}"
    );
    if !repairs.is_empty() {
        let _ = writeln!(rendered_text, "repairs: {}", repairs.join(", "));
        if let Some(cq) = canonical_query.as_deref() {
            let _ = writeln!(rendered_text, "canonical_query: {cq}");
        }
    }

    let known_paths: HashSet<&str> = snapshot.files.iter().map(|f| f.path.as_str()).collect();
    let file_id_map: HashMap<usize, &str> = snapshot
        .files
        .iter()
        .filter_map(|f| {
            usize::try_from(f.file_id)
                .ok()
                .map(|id| (id, f.path.as_str()))
        })
        .collect();

    if tag == "choice-budget-exhausted" && payload.len() >= 3 {
        let budget = decode_usize(native(payload.element(1))?)?;
        let raw_msg = decode_string(native(payload.element(2))?, MAX_BUDGET_MSG_BYTES)?;
        let mut sample_worlds = Vec::new();
        let mut rows = Vec::new();
        let summary = if payload.len() >= 11 {
            let scoped_items = decode_usize(native(payload.element(3))?)?;
            let constrained_items = decode_usize(native(payload.element(4))?)?;
            let free_vars = decode_usize(native(payload.element(5))?)?;
            let branch_factor = decode_usize(native(payload.element(6))?)?;
            let estimated_worlds = decode_usize(native(payload.element(7))?)?;
            let recommended_limit = decode_usize(native(payload.element(8))?)?;
            let sample_scope_count = decode_usize(native(payload.element(9))?)?;
            let sample_vec =
                native(native(payload.element(10))?.into_collection(CollectionKind::Vector))?;
            for i in 0..sample_vec.len() {
                sample_worlds.push(decode_choice_world(native(sample_vec.element(i))?)?);
            }
            let est_label = if estimated_worlds >= 1_000_000 {
                ">=1000000".to_owned()
            } else {
                estimated_worlds.to_string()
            };
            let detail = format!(
                "state-budget {budget} exhausted: scoped_items={scoped_items}, constrained={constrained_items}, free_choice_vars={free_vars}, branch_factor={branch_factor}, estimated_worlds={est_label}; narrow to (scope ... limit: {recommended_limit}) or raise state-budget: (e.g., 256+)"
            );
            let _ = writeln!(
                rendered_text,
                "status: budget-exhausted\ncombinatorics: scoped_items={scoped_items}, constrained={constrained_items}, free_choice_vars={free_vars}, branch_factor={branch_factor}, estimated_worlds={est_label}, state_budget={budget}\nrecommendation: retry with (scope ... limit: {recommended_limit}) or raise state-budget: (or add tighter constraints)"
            );
            if !sample_worlds.is_empty() {
                let _ = writeln!(
                    rendered_text,
                    "partial_witness_sample: deterministic subset ({sample_scope_count}/{scoped_items} scoped items -> {} SAT world(s) verified):",
                    sample_worlds.len()
                );
                for world in &sample_worlds {
                    append_world_rows(
                        world,
                        &known_paths,
                        &file_id_map,
                        ctx.output_limit,
                        &mut rendered_text,
                        &mut rows,
                    );
                }
            }
            detail
        } else {
            let detail = format!(
                "state-budget {budget} exhausted; narrow with (use ...) or (scope path-prefix: \"...\" limit: N), or raise state-budget: ({})",
                raw_msg.lines().next().unwrap_or("budget exhausted").trim()
            );
            let _ = writeln!(rendered_text, "status: budget-exhausted\ndetail: {detail}");
            detail
        };
        if rows.is_empty() {
            rows.push(SearchDslStructuredRow {
                cells: vec!["budget-exhausted".to_owned(), budget.to_string()],
                display: format!("[Dusa budget exhausted: {budget}] {summary}"),
                path: None,
                line: None,
            });
        }
        return Ok(SearchDslStructuredResult {
            generation_digest: snapshot.generation_digest.as_str().to_owned(),
            elapsed_micros: snapshot.elapsed_micros,
            indexed_files: snapshot.indexed_files,
            matched_files: snapshot.files.len(),
            mode: SearchDslExecutionMode::Choice,
            relation,
            repairs,
            canonical_query,
            rows,
            choice_worlds: sample_worlds,
            budget_exhausted: Some(summary),
            rendered_text,
        });
    }

    if tag != "choice-report" || (payload.len() != 5 && payload.len() != 7) {
        return Err(SearchPlanProjectionError::Invalid(
            "unexpected Dusa choice payload shape",
        ));
    }

    let world_count = decode_usize(native(payload.element(1))?)?;
    let state_budget = decode_usize(native(payload.element(2))?)?;
    let provenance_on = matches!(
        native(native(payload.element(3))?.as_ref().scalar())?,
        SchemeScalar::Boolean(true)
    );
    let worlds_vec = native(native(payload.element(4))?.into_collection(CollectionKind::Vector))?;
    let module_slice_counts = if payload.len() == 7 {
        Some((
            decode_usize(native(payload.element(5))?)?,
            decode_usize(native(payload.element(6))?)?,
        ))
    } else {
        None
    };
    let mut choice_worlds = Vec::with_capacity(worlds_vec.len());
    for i in 0..worlds_vec.len() {
        choice_worlds.push(decode_choice_world(native(worlds_vec.element(i))?)?);
    }

    let sat_label = if choice_worlds.is_empty() {
        "UNSAT (constraints are mutually unsatisfiable under active EDB facts)"
    } else {
        "SAT"
    };
    const MAX_DISPLAY_WORLDS: usize = 4;
    let display_truncated = choice_worlds.len() > MAX_DISPLAY_WORLDS;
    let _ = writeln!(
        rendered_text,
        "worlds: {world_count} ({sat_label}, worlds_total_known: true, worlds_truncated: false, display_truncated: {display_truncated}, state-budget: {state_budget}, provenance: {provenance_on})"
    );
    if let Some((slice_modules, total_scoped)) = module_slice_counts {
        let _ = writeln!(
            rendered_text,
            "module_slice: {slice_modules}/{total_scoped} scoped file(s) (hot_path_source: path-predicate 100% file coverage, independent of native_ast_files)\nhot_path_predicate: path_contains(search-dsl | edb_bridge | converged_resident_index | c-abi | span_overlay | linked_ui | hot_path | hot-path | kernel)"
        );
    }
    if let Some(partition) = summarize_forced_and_free_bindings(&choice_worlds) {
        let _ = writeln!(rendered_text, "deduction_vs_branch: {partition}");
    }

    let mut rows = Vec::new();
    let display_indices: Vec<usize> = if display_truncated {
        vec![0, 1, 2, choice_worlds.len() - 1]
    } else {
        (0..choice_worlds.len()).collect()
    };
    for &idx in &display_indices {
        append_world_rows(
            &choice_worlds[idx],
            &known_paths,
            &file_id_map,
            ctx.output_limit,
            &mut rendered_text,
            &mut rows,
        );
    }
    if display_truncated {
        let omitted_worlds = choice_worlds.len().saturating_sub(display_indices.len());
        let _ = writeln!(
            rendered_text,
            "... (+{omitted_worlds} additional SAT world(s) omitted from display; all {world_count} worlds are exhaustively summarized in `deduction_vs_branch` above)"
        );
    }

    Ok(SearchDslStructuredResult {
        generation_digest: snapshot.generation_digest.as_str().to_owned(),
        elapsed_micros: snapshot.elapsed_micros,
        indexed_files: snapshot.indexed_files,
        matched_files: snapshot.files.len(),
        mode: SearchDslExecutionMode::Choice,
        relation,
        repairs,
        canonical_query,
        rows,
        choice_worlds,
        budget_exhausted: None,
        rendered_text,
    })
}

#[cfg(test)]
#[path = "../../tests/unit/choice_codec_provenance.rs"]
mod choice_codec_provenance_tests;
