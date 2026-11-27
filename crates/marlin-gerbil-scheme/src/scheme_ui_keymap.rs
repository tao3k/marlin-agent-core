// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Direct native Gerbil Scheme -> Rust C-ABI FFI decoder for `marlin.ui.v1`
//! commands, dual keymap (`SPC` <-> `C-x`/`C-c`/`C-h`/`M-`), and Magit transients.
//!
//! Calls `:marlin-agent-core/ui-magit/funcs#marlin-ui-magit-project-grpc-surface`
//! directly in-process via `gerbil_scheme::GerbilRuntime` — zero text serialization
//! and zero duplicated Rust/TOML keymap tables.

use std::collections::HashMap;

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierKind, ObjectKind, RootedSchemeCollection,
    RootedSchemeObject, SchemeScalar,
};
use marlin_agent_app_server::wire::marlin::{
    projection::v1::{GraphRecordView, GraphVisibleSurface},
    ui::v1::{
        UiCommand, UiCommandArguments, UiElement, UiKeyBinding, UiKeyScope, UiSpan, UiTextNode,
        UiTone, UiTransient, UiTransientForeign, UiTransientHead, ui_element,
    },
};

/// Native projection output decoded directly from Gerbil Scheme objects.
#[derive(Clone, Debug, Default)]
pub struct ProjectedSchemeUiSurface {
    pub commands: Vec<UiCommand>,
    pub keymap: Vec<UiKeyBinding>,
    pub transients: Vec<UiTransient>,
    pub mode_records: Vec<GraphRecordView>,
}

fn decode_string(obj: RootedSchemeObject<'_>, field: &str) -> Result<String, String> {
    if obj.as_ref().kind().map_err(|e| format!("{field}: {e}"))? != ObjectKind::String {
        return Err(format!("{field}: expected Scheme string"));
    }
    let s = obj.into_string().map_err(|e| format!("{field}: {e}"))?;
    s.to_string()
        .into_result()
        .map_err(|e| format!("{field}: {e}"))
}

fn decode_bool(obj: RootedSchemeObject<'_>, field: &str) -> Result<bool, String> {
    match obj
        .as_ref()
        .scalar()
        .map_err(|e| format!("{field}: {e}"))?
    {
        SchemeScalar::Boolean(b) => Ok(b),
        _ => Err(format!("{field}: expected Scheme boolean")),
    }
}

fn decode_vector<'a>(
    obj: RootedSchemeObject<'a>,
    field: &str,
) -> Result<RootedSchemeCollection<'a>, String> {
    let coll = obj
        .into_collection(CollectionKind::Vector)
        .map_err(|e| format!("{field}: expected Scheme vector: {e}"))?;
    Ok(coll)
}

fn hint_text(title: &str) -> UiElement {
    UiElement {
        node: Some(ui_element::Node::Text(UiTextNode {
            spans: vec![UiSpan {
                text: title.to_owned(),
                tone: UiTone::Accent as i32,
            }],
        })),
    }
}

/// Call `:marlin-agent-core/ui-magit/funcs#marlin-ui-magit-project-grpc-surface`
/// over the in-process Gerbil VM and decode the returned Scheme vector directly
/// into `marlin.ui.v1` protobuf types.
#[cfg(not(feature = "external-program"))]
pub fn project_scheme_ui_surface(
    runtime: &GerbilRuntime,
    scheme_package_dir: &str,
) -> Result<ProjectedSchemeUiSurface, String> {
    let module = runtime
        .intern_identifier(IdentifierKind::Symbol, ":marlin-agent-core/ui-magit/funcs")
        .map_err(|e| format!("intern ui-magit module: {e}"))?;
    let export_name = runtime
        .intern_identifier(
            IdentifierKind::Symbol,
            "marlin-ui-magit-project-grpc-surface",
        )
        .map_err(|e| format!("intern projection export: {e}"))?;
    let proc_obj = runtime
        .resolve_package_export(scheme_package_dir, &module, &export_name)
        .map_err(|e| format!("resolve marlin-ui-magit-project-grpc-surface: {e}"))?;
    let dummy_input = runtime
        .scalar_object(SchemeScalar::Boolean(false))
        .map_err(|e| format!("scalar input: {e}"))?;
    let raw = runtime
        .call_rooted(&proc_obj, &dummy_input)
        .map_err(|e| format!("call marlin-ui-magit-project-grpc-surface: {e}"))?;
    let mut surface = decode_projected_surface_vector(raw)?;
    if let Ok(records) = project_scheme_modes_records(runtime, scheme_package_dir) {
        surface.mode_records = records;
    }
    Ok(surface)
}

fn decode_fixnum(obj: RootedSchemeObject<'_>, field: &str) -> Result<i64, String> {
    match obj
        .as_ref()
        .scalar()
        .map_err(|e| format!("{field}: {e}"))?
    {
        SchemeScalar::Fixnum(n) => Ok(n as i64),
        _ => Err(format!("{field}: expected Scheme fixnum")),
    }
}

fn decode_string_vec(obj: RootedSchemeObject<'_>, field: &str) -> Result<Vec<String>, String> {
    let vec = decode_vector(obj, field)?;
    let mut out = Vec::with_capacity(vec.len());
    for i in 0..vec.len() {
        out.push(decode_string(
            vec.element(i).map_err(|e| e.to_string())?,
            field,
        )?);
    }
    Ok(out)
}

fn decode_mode_bindings_compact(
    obj: RootedSchemeObject<'_>,
    field: &str,
) -> Result<String, String> {
    let vec = decode_vector(obj, field)?;
    let mut parts = Vec::with_capacity(vec.len());
    for i in 0..vec.len() {
        let entry = decode_vector(vec.element(i).map_err(|e| e.to_string())?, field)?;
        if entry.len() != 4 {
            return Err(format!("{field}[{i}]: expected 4 elements"));
        }
        let k = decode_string(entry.element(0).map_err(|e| e.to_string())?, "bind.key")?;
        let c = decode_string(entry.element(1).map_err(|e| e.to_string())?, "bind.cmd")?;
        let l = decode_string(entry.element(2).map_err(|e| e.to_string())?, "bind.label")?;
        let ex = decode_bool(entry.element(3).map_err(|e| e.to_string())?, "bind.exit")?;
        parts.push(format!("{k}\t{c}\t{l}\t{ex}"));
    }
    Ok(parts.join("\n"))
}

fn make_mode_record(kind: &str, id: &str, title: &str, facets: HashMap<String, String>) -> GraphRecordView {
    GraphRecordView {
        record_id: format!("{kind}:{id}"),
        surface: GraphVisibleSurface::Workspace as i32,
        kind: kind.to_owned(),
        title: title.to_owned(),
        summary: String::new(),
        source_path: None,
        start_line: None,
        end_line: None,
        facets,
    }
}

/// Call `:marlin-agent-core/ui-modes/funcs#marlin-modes-project-grpc-registry`
/// over the in-process Gerbil VM and decode Major Modes, Minor Modes, Inner Modes,
/// Polymodes, and Buffer-to-Mode routing rules into typed `GraphRecordView` records.
#[cfg(not(feature = "external-program"))]
pub fn project_scheme_modes_records(
    runtime: &GerbilRuntime,
    scheme_package_dir: &str,
) -> Result<Vec<GraphRecordView>, String> {
    let module = runtime
        .intern_identifier(IdentifierKind::Symbol, ":marlin-agent-core/ui-modes/funcs")
        .map_err(|e| format!("intern ui-modes module: {e}"))?;
    let export_name = runtime
        .intern_identifier(IdentifierKind::Symbol, "marlin-modes-project-grpc-registry")
        .map_err(|e| format!("intern modes projection export: {e}"))?;
    let proc_obj = runtime
        .resolve_package_export(scheme_package_dir, &module, &export_name)
        .map_err(|e| format!("resolve marlin-modes-project-grpc-registry: {e}"))?;
    let dummy_input = runtime
        .scalar_object(SchemeScalar::Boolean(false))
        .map_err(|e| format!("scalar input: {e}"))?;
    let raw = runtime
        .call_rooted(&proc_obj, &dummy_input)
        .map_err(|e| format!("call marlin-modes-project-grpc-registry: {e}"))?;
    decode_modes_registry_vector(raw)
}

/// Decode `#(major-modes minor-modes inner-modes polymodes buffer-rules)` from Scheme.
pub fn decode_modes_registry_vector(
    raw: RootedSchemeObject<'_>,
) -> Result<Vec<GraphRecordView>, String> {
    let root = decode_vector(raw, "modes-registry")?;
    if root.len() != 5 {
        return Err(format!("modes-registry: expected 5 elements, got {}", root.len()));
    }
    let mut records = Vec::new();

    for (idx, kind) in [(0usize, "ui.mode.major"), (2usize, "ui.mode.inner")] {
        let vec = decode_vector(root.element(idx).map_err(|e| e.to_string())?, kind)?;
        for i in 0..vec.len() {
            let m = decode_vector(vec.element(i).map_err(|e| e.to_string())?, kind)?;
            if m.len() != 10 {
                return Err(format!("{kind}[{i}]: expected 10 elements"));
            }
            let id = decode_string(m.element(0).map_err(|e| e.to_string())?, "mode.id")?;
            let title = decode_string(m.element(1).map_err(|e| e.to_string())?, "mode.title")?;
            let mut facets = HashMap::new();
            facets.insert("id".to_owned(), id.clone());
            facets.insert("parent".to_owned(), decode_string(m.element(2).map_err(|e| e.to_string())?, "mode.parent")?);
            facets.insert("read_only".to_owned(), decode_bool(m.element(3).map_err(|e| e.to_string())?, "mode.ro")?.to_string());
            facets.insert("display_action".to_owned(), decode_string(m.element(4).map_err(|e| e.to_string())?, "mode.display")?);
            facets.insert("dedicated".to_owned(), decode_bool(m.element(5).map_err(|e| e.to_string())?, "mode.dedicated")?.to_string());
            facets.insert("keymap".to_owned(), decode_mode_bindings_compact(m.element(6).map_err(|e| e.to_string())?, "mode.keymap")?);
            facets.insert("default_minors".to_owned(), decode_string_vec(m.element(7).map_err(|e| e.to_string())?, "mode.minors")?.join(","));
            facets.insert("modeline_segments".to_owned(), decode_string_vec(m.element(8).map_err(|e| e.to_string())?, "mode.modeline")?.join(","));
            facets.insert("new_actions".to_owned(), decode_string_vec(m.element(9).map_err(|e| e.to_string())?, "mode.actions")?.join(","));
            records.push(make_mode_record(kind, &id, &title, facets));
        }
    }

    let minors = decode_vector(root.element(1).map_err(|e| e.to_string())?, "ui.mode.minor")?;
    for i in 0..minors.len() {
        let m = decode_vector(minors.element(i).map_err(|e| e.to_string())?, "ui.mode.minor")?;
        if m.len() != 5 {
            return Err(format!("ui.mode.minor[{i}]: expected 5 elements"));
        }
        let id = decode_string(m.element(0).map_err(|e| e.to_string())?, "minor.id")?;
        let title = decode_string(m.element(1).map_err(|e| e.to_string())?, "minor.title")?;
        let mut facets = HashMap::new();
        facets.insert("id".to_owned(), id.clone());
        facets.insert("lighter".to_owned(), decode_string(m.element(2).map_err(|e| e.to_string())?, "minor.lighter")?);
        facets.insert("priority".to_owned(), decode_fixnum(m.element(3).map_err(|e| e.to_string())?, "minor.prio")?.to_string());
        facets.insert("keymap".to_owned(), decode_mode_bindings_compact(m.element(4).map_err(|e| e.to_string())?, "minor.keymap")?);
        records.push(make_mode_record("ui.mode.minor", &id, &title, facets));
    }

    let polys = decode_vector(root.element(3).map_err(|e| e.to_string())?, "ui.polymode")?;
    for i in 0..polys.len() {
        let p = decode_vector(polys.element(i).map_err(|e| e.to_string())?, "ui.polymode")?;
        if p.len() != 3 {
            return Err(format!("ui.polymode[{i}]: expected 3 elements"));
        }
        let id = decode_string(p.element(0).map_err(|e| e.to_string())?, "poly.id")?;
        let host = decode_string(p.element(1).map_err(|e| e.to_string())?, "poly.host")?;
        let rules_vec = decode_vector(p.element(2).map_err(|e| e.to_string())?, "poly.rules")?;
        let mut rules_lines = Vec::with_capacity(rules_vec.len());
        for j in 0..rules_vec.len() {
            let r = decode_vector(rules_vec.element(j).map_err(|e| e.to_string())?, "poly.rule")?;
            if r.len() != 3 {
                return Err(format!("ui.polymode[{i}].rule[{j}]: expected 3 elements"));
            }
            let span = decode_string(r.element(0).map_err(|e| e.to_string())?, "poly.span")?;
            let inner = decode_string(r.element(1).map_err(|e| e.to_string())?, "poly.inner")?;
            let lighter = decode_string(r.element(2).map_err(|e| e.to_string())?, "poly.lighter")?;
            rules_lines.push(format!("{span}\t{inner}\t{lighter}"));
        }
        let mut facets = HashMap::new();
        facets.insert("id".to_owned(), id.clone());
        facets.insert("host_mode".to_owned(), host.clone());
        facets.insert("inner_rules".to_owned(), rules_lines.join("\n"));
        records.push(make_mode_record("ui.polymode", &id, &host, facets));
    }

    let rules = decode_vector(root.element(4).map_err(|e| e.to_string())?, "ui.buffer_rule")?;
    for i in 0..rules.len() {
        let r = decode_vector(rules.element(i).map_err(|e| e.to_string())?, "ui.buffer_rule")?;
        if r.len() != 5 {
            return Err(format!("ui.buffer_rule[{i}]: expected 5 elements"));
        }
        let prefix = decode_string(r.element(0).map_err(|e| e.to_string())?, "rule.prefix")?;
        let mode = decode_string(r.element(1).map_err(|e| e.to_string())?, "rule.mode")?;
        let creatable = decode_bool(r.element(2).map_err(|e| e.to_string())?, "rule.creatable")?;
        let label = decode_string(r.element(3).map_err(|e| e.to_string())?, "rule.label")?;
        let action = decode_string(r.element(4).map_err(|e| e.to_string())?, "rule.action")?;
        let mut facets = HashMap::new();
        facets.insert("prefix".to_owned(), prefix.clone());
        facets.insert("major_mode".to_owned(), mode);
        facets.insert("creatable".to_owned(), creatable.to_string());
        facets.insert("create_label".to_owned(), label.clone());
        facets.insert("create_action".to_owned(), action);
        records.push(make_mode_record("ui.buffer_rule", &prefix, &label, facets));
    }

    Ok(records)
}

/// Decode `#(commands-vec keymap-vec transients-vec)` returned by Scheme.
pub fn decode_projected_surface_vector(
    raw: RootedSchemeObject<'_>,
) -> Result<ProjectedSchemeUiSurface, String> {
    let root = decode_vector(raw, "surface")?;
    if root.len() != 3 {
        return Err(format!(
            "surface: expected 3-element vector #(commands keymap transients), got {}",
            root.len()
        ));
    }

    // 0: commands -> #(#(name desc capability) ...)
    let cmds_vec = decode_vector(root.element(0).map_err(|e| e.to_string())?, "commands")?;
    let mut commands = Vec::with_capacity(cmds_vec.len());
    for i in 0..cmds_vec.len() {
        let entry = decode_vector(
            cmds_vec.element(i).map_err(|e| e.to_string())?,
            "command entry",
        )?;
        if entry.len() != 3 {
            return Err(format!("command[{i}]: expected 3 elements"));
        }
        commands.push(UiCommand {
            name: decode_string(entry.element(0).map_err(|e| e.to_string())?, "cmd.name")?,
            description: decode_string(entry.element(1).map_err(|e| e.to_string())?, "cmd.desc")?,
            arguments: UiCommandArguments::None as i32,
            capability: decode_string(entry.element(2).map_err(|e| e.to_string())?, "cmd.cap")?,
            fixed_value: None,
        });
    }

    // 1: keymap -> #(#(scope #(k1 k2 ...) cmd) ...)
    let km_vec = decode_vector(root.element(1).map_err(|e| e.to_string())?, "keymap")?;
    let mut keymap = Vec::with_capacity(km_vec.len());
    for i in 0..km_vec.len() {
        let entry = decode_vector(
            km_vec.element(i).map_err(|e| e.to_string())?,
            "keymap entry",
        )?;
        if entry.len() != 3 {
            return Err(format!("keymap[{i}]: expected 3 elements"));
        }
        let scope_str = decode_string(entry.element(0).map_err(|e| e.to_string())?, "km.scope")?;
        let scope = match scope_str.as_str() {
            "global" => UiKeyScope::Global as i32,
            "insert" => UiKeyScope::Insert as i32,
            _ => UiKeyScope::Normal as i32,
        };
        let seq_vec = decode_vector(entry.element(1).map_err(|e| e.to_string())?, "km.seq")?;
        let mut sequence = Vec::with_capacity(seq_vec.len());
        for j in 0..seq_vec.len() {
            sequence.push(decode_string(
                seq_vec.element(j).map_err(|e| e.to_string())?,
                "km.seq.key",
            )?);
        }
        let command_name = decode_string(entry.element(2).map_err(|e| e.to_string())?, "km.cmd")?;
        keymap.push(UiKeyBinding {
            scope,
            sequence,
            action_kind: "invoke".to_owned(),
            command_name: Some(command_name),
        });
    }

    // 2: transients -> #(#(prefix title #(#(key cmd label exit?) ...)) ...)
    let tr_vec = decode_vector(root.element(2).map_err(|e| e.to_string())?, "transients")?;
    let mut transients = Vec::with_capacity(tr_vec.len());
    for i in 0..tr_vec.len() {
        let entry = decode_vector(
            tr_vec.element(i).map_err(|e| e.to_string())?,
            "transient entry",
        )?;
        if entry.len() != 3 {
            return Err(format!("transient[{i}]: expected 3 elements"));
        }
        let id = decode_string(entry.element(0).map_err(|e| e.to_string())?, "tr.id")?;
        let title = decode_string(entry.element(1).map_err(|e| e.to_string())?, "tr.title")?;
        let heads_vec = decode_vector(entry.element(2).map_err(|e| e.to_string())?, "tr.heads")?;
        let mut heads = Vec::with_capacity(heads_vec.len());
        for j in 0..heads_vec.len() {
            let hd = decode_vector(
                heads_vec.element(j).map_err(|e| e.to_string())?,
                "tr.head",
            )?;
            if hd.len() != 4 {
                return Err(format!("transient[{i}].head[{j}]: expected 4 elements"));
            }
            let key = decode_string(hd.element(0).map_err(|e| e.to_string())?, "head.key")?;
            let cmd = decode_string(hd.element(1).map_err(|e| e.to_string())?, "head.cmd")?;
            let label = decode_string(hd.element(2).map_err(|e| e.to_string())?, "head.label")?;
            let exit = decode_bool(hd.element(3).map_err(|e| e.to_string())?, "head.exit")?;
            heads.push(UiTransientHead {
                key,
                command_name: if cmd.is_empty() { None } else { Some(cmd) },
                label,
                exit,
            });
        }
        transients.push(UiTransient {
            id,
            hint: Some(hint_text(&title)),
            warning_hint: Some(hint_text(&format!("{title} — press a listed key or q"))),
            foreign: UiTransientForeign::Exit as i32,
            heads,
        });
    }

    Ok(ProjectedSchemeUiSurface {
        commands,
        keymap,
        transients,
        mode_records: Vec::new(),
    })
}
