// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Scheme-authored window scenarios executed by the production frame mapper.

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, ObjectKind,
    RootedSchemeObject, SchemeScalar,
};
use marlin_agent_ui::{UiComponentId, UiProjectionError, decode_workbench};
use ratatui::{Terminal, backend::TestBackend};

/// A scenario receives `#(rows hit-regions cursor-position pointer-hits)`, initially void.
/// It returns `#f` on success or `#(width height workbench pointer-probes [bindings])`.
/// Bindings supply one frame of named text inputs as a vector of `#(id text)`.
/// Scheme owns assertions and state transitions; this driver only maps frames.
/// The frame and step bounds also apply to failing package scenarios.
pub fn run(
    runtime: &GerbilRuntime,
    scenario: &RootedSchemeObject<'_>,
) -> Result<usize, UiProjectionError> {
    let mut snapshot = runtime.void_object()?;
    let mut cache = Default::default();
    let mut anchor = Default::default();
    let mut app = crate::app::App::new(marlin_agent_protocol::ModelEndpoint::new("test", "test"));
    for step in 0..256 {
        let request = runtime.call_rooted(scenario, &snapshot)?;
        if matches!(request.as_ref().scalar(), Ok(SchemeScalar::Boolean(false))) {
            return Ok(step);
        }
        let request = request.into_collection(CollectionKind::Vector)?;
        if !(4..=5).contains(&request.len()) {
            return Err(UiProjectionError::Invalid("invalid test window request"));
        }
        let dimension = |index| -> Result<u16, UiProjectionError> {
            match request.element(index)?.as_ref().scalar()? {
                SchemeScalar::Fixnum(value) if (1..=512).contains(&value) => Ok(value as u16),
                _ => Err(UiProjectionError::Invalid("invalid test window dimension")),
            }
        };
        let (width, height) = (dimension(0)?, dimension(1)?);
        let view = decode_workbench(request.element(2)?)?;
        let probes = request
            .element(3)?
            .into_collection(CollectionKind::Vector)?;
        if probes.len() > 16 {
            return Err(UiProjectionError::Invalid("too many pointer probes"));
        }
        let bindings = if request.len() == 5 {
            frame_bindings(request.element(4)?)?
        } else {
            Vec::new()
        };
        app.sync_window(&view).map_err(UiProjectionError::Invalid)?;
        let geometry = crate::render::frame_snapshot_with_bindings(
            &app,
            &view,
            ratatui::layout::Rect::new(0, 0, width, height),
            bindings,
        );
        let mut terminal = Terminal::new(TestBackend::new(width, height))
            .map_err(|_| UiProjectionError::Invalid("test window creation failed"))?;
        let mut painted = Vec::new();
        terminal
            .draw(|frame| {
                painted = crate::render::draw_with_snapshot(
                    frame,
                    &app,
                    &view,
                    &mut cache,
                    &mut anchor,
                    &geometry,
                )
                .regions;
            })
            .map_err(|_| UiProjectionError::Invalid("test window draw failed"))?;
        let buffer = terminal.backend().buffer();
        let rows: Result<Vec<_>, _> = (0..height)
            .map(|y| {
                let text: String = (0..width).map(|x| buffer[(x, y)].symbol()).collect();
                runtime
                    .string_from_utf8(&text)
                    .map(|value| value.into_object())
            })
            .collect();
        let rows = rows?;
        let rows = values(runtime, rows)?;
        let mut regions = Vec::with_capacity(painted.len());
        for region in &painted {
            let id = runtime.intern_identifier(IdentifierKind::Symbol, region.id.as_str())?;
            let target = match region.scroll_target() {
                Some(target) => runtime.intern_identifier(IdentifierKind::Symbol, target)?,
                None => runtime.scalar_object(SchemeScalar::Boolean(false))?,
            };
            let mut fields = vec![id];
            for value in [
                region.area.x,
                region.area.y,
                region.area.width,
                region.area.height,
                region.scroll_step().map_or(0, |step| step),
            ] {
                fields.push(runtime.scalar_object(SchemeScalar::Fixnum(value as isize))?);
            }
            fields.push(target);
            regions.push(values(runtime, fields)?);
        }
        let regions = values(runtime, regions)?;
        let cursor = terminal
            .get_cursor_position()
            .map_err(|_| UiProjectionError::Invalid("test window cursor failed"))?;
        let cursor = values(
            runtime,
            vec![
                runtime.scalar_object(SchemeScalar::Fixnum(cursor.x as isize))?,
                runtime.scalar_object(SchemeScalar::Fixnum(cursor.y as isize))?,
            ],
        )?;
        let mut hits = Vec::with_capacity(probes.len());
        for index in 0..probes.len() {
            let probe = probes
                .element(index)?
                .into_collection(CollectionKind::Vector)?;
            if probe.len() != 2 {
                return Err(UiProjectionError::Invalid("invalid pointer probe"));
            }
            let cell = |field| -> Result<u16, UiProjectionError> {
                match probe.element(field)?.as_ref().scalar()? {
                    SchemeScalar::Fixnum(value) if (0..512).contains(&value) => Ok(value as u16),
                    _ => Err(UiProjectionError::Invalid("invalid pointer cell")),
                }
            };
            let (x, y) = (cell(0)?, cell(1)?);
            let region = crate::ui_elements::hit_test(&painted, x, y);
            let wheel = crate::ui_elements::hit_test_scroll(&painted, x, y);
            let lists = app.list_state.borrow();
            let row = lists.hit(x, y);
            let id = |value: Option<&str>| match value {
                Some(value) => runtime.intern_identifier(IdentifierKind::Symbol, value),
                None => runtime.scalar_object(SchemeScalar::Boolean(false)),
            };
            hits.push(values(
                runtime,
                vec![
                    id(region.map(|hit| hit.id.as_str()))?,
                    id(wheel.and_then(|hit| hit.scroll_target()))?,
                    id(row.map(|hit| hit.list.as_str()))?,
                    id(row.map(|hit| hit.key.as_str()))?,
                ],
            )?);
        }
        let hits = values(runtime, hits)?;
        snapshot = values(runtime, vec![rows, regions, cursor, hits])?;
    }
    Err(UiProjectionError::Invalid(
        "test window step limit exceeded",
    ))
}

fn frame_bindings(
    input: RootedSchemeObject<'_>,
) -> Result<Vec<(UiComponentId, String)>, UiProjectionError> {
    let input = input.into_collection(CollectionKind::Vector)?;
    if input.len() > 16 {
        return Err(UiProjectionError::Invalid("too many frame bindings"));
    }
    let mut bindings = Vec::with_capacity(input.len());
    let mut names = IdentifierBuffer::default();
    for index in 0..input.len() {
        let entry = input
            .element(index)?
            .into_collection(CollectionKind::Vector)?;
        if entry.len() != 2 {
            return Err(UiProjectionError::Invalid("invalid frame binding"));
        }
        let key = entry.element(0)?;
        if key.as_ref().kind()? != ObjectKind::Symbol {
            return Err(UiProjectionError::Invalid("expected frame binding symbol"));
        }
        key.as_ref().copy_identifier_names(&mut names)?;
        let Some((IdentifierKind::Symbol, name)) = names.get(0) else {
            return Err(UiProjectionError::Invalid("invalid frame binding symbol"));
        };
        let name = UiComponentId::new(name)?;
        if bindings.iter().any(|(existing, _)| existing == &name) {
            return Err(UiProjectionError::Invalid("duplicate frame binding"));
        }
        let text = entry.element(1)?.into_string()?;
        if text.len().into_result()? > 256 {
            return Err(UiProjectionError::Invalid(
                "frame binding text exceeds limit",
            ));
        }
        bindings.push((
            name,
            crate::app::safe_text(&text.to_string().into_result()?),
        ));
    }
    Ok(bindings)
}

fn values<'a>(
    runtime: &'a GerbilRuntime,
    items: Vec<RootedSchemeObject<'a>>,
) -> Result<RootedSchemeObject<'a>, UiProjectionError> {
    let refs: Vec<_> = items.iter().collect();
    Ok(runtime
        .collection_from_objects(CollectionKind::Vector, &refs)?
        .into_object())
}
