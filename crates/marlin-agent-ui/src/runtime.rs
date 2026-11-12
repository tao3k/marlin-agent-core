// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! SDK-rooted Scheme UI session and native call boundary.

use std::collections::HashSet;

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierKind, LinkedObjectExport, ObjectKind,
    RootedObjectBatch, RootedSchemeObject, SchemeScalar,
};

use crate::model::{
    MAX_EXTENSIONS, MAX_METHOD_BUNDLES, MAX_STATUS, MAX_TEXT_CHARS, UiBrowseDirection,
    UiCommandName, UiEvent, UiIntent, UiOpenFacts, UiProjectionError, UiWorkbench,
};
use crate::projection::{decode_intent, decode_workbench, string, vector};

/// Scheme-owned UI state, retained by the SDK's owner-affine strong root.
pub struct UiNativeSession<'runtime> {
    root: RootedSchemeObject<'runtime>,
}

/// SDK-owned module exports for one selected Marlin UI publication.
/// Resolve at startup before concurrent environment or subprocess work begins.
pub struct UiModule<'runtime> {
    runtime: &'runtime GerbilRuntime,
    publication: RootedSchemeObject<'runtime>,
    open: UiCallable<'runtime>,
    project: UiCallable<'runtime>,
    dispatch: UiCallable<'runtime>,
    browse_command: UiCallable<'runtime>,
    refresh_labels: UiCallable<'runtime>,
}

enum UiCallable<'runtime> {
    #[cfg(not(feature = "linked-ui"))]
    Dynamic(RootedSchemeObject<'runtime>),
    Linked(LinkedObjectExport<'runtime>),
}

impl<'runtime> UiCallable<'runtime> {
    fn call(
        &self,
        _runtime: &'runtime GerbilRuntime,
        input: &RootedSchemeObject<'_>,
    ) -> Result<RootedSchemeObject<'runtime>, UiProjectionError> {
        match self {
            #[cfg(not(feature = "linked-ui"))]
            Self::Dynamic(procedure) => Ok(_runtime.call_rooted(procedure, input)?),
            Self::Linked(export) => Ok(export.call(input).into_result()?),
        }
    }
}

/// AOT exports from the same Yuanxu-selected programme as the VM linker.
/// Each export uses the SDK strong-root object ABI and must belong to that
/// programme; constructing a module from arbitrary function pointers is unsafe.
#[derive(Clone, Copy)]
pub struct UiLinkedExports {
    pub publication: unsafe extern "C" fn(i64) -> i64,
    pub open: unsafe extern "C" fn(i64) -> i64,
    pub project: unsafe extern "C" fn(i64) -> i64,
    pub dispatch: unsafe extern "C" fn(i64) -> i64,
    pub browse_command: unsafe extern "C" fn(i64) -> i64,
    pub refresh_labels: unsafe extern "C" fn(i64) -> i64,
}

impl<'runtime> UiModule<'runtime> {
    /// Resolve the declared publication and UI procedure exports in one VM.
    #[cfg(not(feature = "linked-ui"))]
    pub fn resolve(
        runtime: &'runtime GerbilRuntime,
        directory: &str,
    ) -> Result<Self, UiProjectionError> {
        let started = std::time::Instant::now();
        let trace = |stage: &str| {
            if std::env::var_os("MARLIN_TUI_STARTUP_TRACE").is_some() {
                eprintln!("marlin-tui UI resolve {stage}: {:?}", started.elapsed());
            }
        };
        fn export<'a>(
            runtime: &'a GerbilRuntime,
            directory: &str,
            module: &str,
            name: &str,
        ) -> Result<RootedSchemeObject<'a>, UiProjectionError> {
            let module = runtime.intern_identifier(IdentifierKind::Symbol, module)?;
            let name = runtime.intern_identifier(IdentifierKind::Symbol, name)?;
            Ok(runtime.resolve_package_export(directory, &module, &name)?)
        }
        let publication = export(
            runtime,
            directory,
            ":marlin-agent-core/init",
            "marlin-agent-core.init.exports",
        )?;
        trace("init publication");
        let ui = ":marlin-agent-core/c-abi/ui";
        let open = export(runtime, directory, ui, "marlin-ui-session-open")?;
        trace("session open");
        let project = export(runtime, directory, ui, "marlin-ui-session-project")?;
        trace("session project");
        let dispatch = export(runtime, directory, ui, "marlin-ui-session-dispatch")?;
        trace("session dispatch");
        let browse_command = export(runtime, directory, ui, "marlin-ui-session-browse-command")?;
        trace("command browse");
        let refresh_labels = export(runtime, directory, ui, "marlin-ui-session-refresh-labels")?;
        trace("label refresh");
        for procedure in [&open, &project, &dispatch, &browse_command, &refresh_labels] {
            if procedure.as_ref().kind()? != ObjectKind::Procedure {
                return Err(UiProjectionError::Invalid("UI export is not a procedure"));
            }
        }
        Ok(Self {
            runtime,
            publication,
            open: UiCallable::Dynamic(open),
            project: UiCallable::Dynamic(project),
            dispatch: UiCallable::Dynamic(dispatch),
            browse_command: UiCallable::Dynamic(browse_command),
            refresh_labels: UiCallable::Dynamic(refresh_labels),
        })
    }

    /// Bind the selected AOT publication and all UI operations to one VM.
    ///
    /// # Safety
    /// Every export must belong to `runtime`'s linked programme, catch Scheme
    /// exceptions and transfer one fresh strong result root. The publication
    /// must be the complete selected `init.exports` value for this UI graph.
    pub unsafe fn resolve_linked(
        runtime: &'runtime GerbilRuntime,
        exports: UiLinkedExports,
    ) -> Result<Self, UiProjectionError> {
        // SAFETY: the caller binds the named exports to the live linked graph.
        let publication_export = unsafe { runtime.bind_object_export(exports.publication)? };
        let publication = publication_export
            .call(&runtime.void_object()?)
            .into_result()?;
        let bind = |export| {
            // SAFETY: the same caller guarantee covers all operation exports.
            unsafe { runtime.bind_object_export(export) }.map(UiCallable::Linked)
        };
        Ok(Self {
            runtime,
            publication,
            open: bind(exports.open)?,
            project: bind(exports.project)?,
            dispatch: bind(exports.dispatch)?,
            browse_command: bind(exports.browse_command)?,
            refresh_labels: bind(exports.refresh_labels)?,
        })
    }

    /// Construct one Scheme-owned functional UI session.
    pub fn open(
        &self,
        facts: &UiOpenFacts<'_, '_>,
    ) -> Result<UiNativeSession<'runtime>, UiProjectionError> {
        open_session_with(self.runtime, &self.publication, facts, |request| {
            self.open.call(self.runtime, request)
        })
    }

    /// Project an existing session into a typed host candidate.
    pub fn project(&self, session: &UiNativeSession<'_>) -> Result<UiWorkbench, UiProjectionError> {
        decode_workbench(self.project.call(self.runtime, &session.root)?)
    }

    /// Ask the selected Scheme generic for a presentation-only command choice.
    pub fn browse_command(
        &self,
        session: &UiNativeSession<'_>,
        query: &str,
        previous: Option<&UiCommandName>,
        direction: UiBrowseDirection,
    ) -> Result<Option<UiCommandName>, UiProjectionError> {
        if !UiCommandName::is_valid_prefix(query) {
            return Err(UiProjectionError::Invalid("invalid command query"));
        }
        let query = self.runtime.string_from_utf8(query)?.into_object();
        let previous = match previous {
            Some(name) => self.runtime.string_from_utf8(name.as_str())?.into_object(),
            None => self.runtime.scalar_object(SchemeScalar::Boolean(false))?,
        };
        let direction = self.runtime.intern_identifier(
            IdentifierKind::Symbol,
            match direction {
                UiBrowseDirection::Next => "next",
                UiBrowseDirection::Previous => "previous",
            },
        )?;
        let input = self.runtime.collection_from_objects(
            CollectionKind::Vector,
            &[&session.root, &query, &previous, &direction],
        )?;
        let output = self
            .browse_command
            .call(self.runtime, &input.into_object())?;
        match output.as_ref().kind()? {
            ObjectKind::String => Ok(Some(UiCommandName::new(&string(output)?)?)),
            ObjectKind::Boolean
                if matches!(output.as_ref().scalar()?, SchemeScalar::Boolean(false)) =>
            {
                Ok(None)
            }
            _ => Err(UiProjectionError::Invalid("invalid command browse result")),
        }
    }

    /// Refresh host-owned display facts while retaining the Scheme UI session.
    pub fn refresh_labels(
        &self,
        session: &UiNativeSession<'_>,
        model: &str,
        session_label: &str,
    ) -> Result<UiNativeSession<'runtime>, UiProjectionError> {
        if model.chars().take(MAX_TEXT_CHARS + 1).count() > MAX_TEXT_CHARS
            || session_label.chars().take(MAX_TEXT_CHARS + 1).count() > MAX_TEXT_CHARS
        {
            return Err(UiProjectionError::Invalid("UI facts exceed limits"));
        }
        let model = self.runtime.string_from_utf8(model)?.into_object();
        let label = self.runtime.string_from_utf8(session_label)?.into_object();
        let request = self
            .runtime
            .collection_from_objects(CollectionKind::Vector, &[&session.root, &model, &label])?;
        Ok(UiNativeSession {
            root: self
                .refresh_labels
                .call(self.runtime, &request.into_object())?,
        })
    }

    /// Dispatch an event without consuming the previous session.
    pub fn dispatch(
        &self,
        session: &UiNativeSession<'runtime>,
        event: UiEvent,
    ) -> Result<(UiNativeSession<'runtime>, UiIntent), UiProjectionError> {
        dispatch_event_with(self.runtime, session, event, |request| {
            self.dispatch.call(self.runtime, request)
        })
    }
}

/// Build a rooted request using the SDK's Scheme string, symbol and vector APIs.
/// The scope root must be the selected Yuanxu publication; the VM owner must
/// establish its and the extensions' provenance.
pub fn prepare_open_request<'runtime>(
    runtime: &'runtime GerbilRuntime,
    scope: &RootedSchemeObject<'_>,
    facts: &UiOpenFacts<'_, '_>,
) -> Result<RootedSchemeObject<'runtime>, UiProjectionError> {
    if facts.model_label.chars().take(MAX_TEXT_CHARS + 1).count() > MAX_TEXT_CHARS
        || facts.session_label.chars().take(MAX_TEXT_CHARS + 1).count() > MAX_TEXT_CHARS
        || facts.status.len() > MAX_STATUS
        || facts.extensions.len() > MAX_EXTENSIONS
        || facts.method_bundles.len() > MAX_METHOD_BUNDLES
    {
        return Err(UiProjectionError::Invalid("UI facts exceed limits"));
    }
    let mut ids = HashSet::new();
    for item in facts.status {
        if item.label.chars().take(MAX_TEXT_CHARS + 1).count() > MAX_TEXT_CHARS
            || matches!(
                item.id.as_str(),
                "transcript" | "composer" | "commands" | "status"
            )
            || !ids.insert(item.id.as_str())
        {
            return Err(UiProjectionError::Invalid("invalid status contribution"));
        }
    }
    let model = runtime.string_from_utf8(facts.model_label)?.into_object();
    let session = runtime.string_from_utf8(facts.session_label)?.into_object();
    let mut rows = Vec::with_capacity(facts.status.len());
    for item in facts.status {
        let id = runtime.intern_identifier(IdentifierKind::Symbol, item.id.as_str())?;
        let label = runtime.string_from_utf8(&item.label)?.into_object();
        rows.push(
            runtime
                .collection_from_objects(CollectionKind::Vector, &[&id, &label])?
                .into_object(),
        );
    }
    let row_refs: Vec<_> = rows.iter().collect();
    let status = runtime
        .collection_from_objects(CollectionKind::Vector, &row_refs)?
        .into_object();
    let extension_refs: Vec<_> = facts.extensions.iter().collect();
    let extensions = runtime
        .collection_from_objects(CollectionKind::Vector, &extension_refs)?
        .into_object();
    let bundle_refs: Vec<_> = facts.method_bundles.iter().collect();
    let bundles = runtime
        .collection_from_objects(CollectionKind::Vector, &bundle_refs)?
        .into_object();
    let request_facts = runtime
        .collection_from_objects(
            CollectionKind::Vector,
            &[&model, &session, &status, &extensions],
        )?
        .into_object();
    Ok(runtime
        .collection_from_objects(CollectionKind::Vector, &[scope, &request_facts, &bundles])?
        .into_object())
}

/// Open a Scheme-owned session using typed host facts and SDK-rooted values.
/// The scope root is the complete Yuanxu publication, not its UI child. The VM
/// owner must establish the export and selected publication's provenance.
pub fn open_session<'runtime>(
    runtime: &'runtime GerbilRuntime,
    export: &RootedSchemeObject<'runtime>,
    scope: &RootedSchemeObject<'_>,
    facts: &UiOpenFacts<'_, '_>,
) -> Result<UiNativeSession<'runtime>, UiProjectionError> {
    open_session_with(runtime, scope, facts, |request| {
        Ok(runtime.call_rooted(export, request)?)
    })
}

fn open_session_with<'runtime>(
    runtime: &'runtime GerbilRuntime,
    scope: &RootedSchemeObject<'_>,
    facts: &UiOpenFacts<'_, '_>,
    call: impl FnOnce(
        &RootedSchemeObject<'_>,
    ) -> Result<RootedSchemeObject<'runtime>, UiProjectionError>,
) -> Result<UiNativeSession<'runtime>, UiProjectionError> {
    let request = prepare_open_request(runtime, scope, facts)?;
    Ok(UiNativeSession {
        root: call(&request)?,
    })
}

/// Project an existing session using the SDK's rooted Scheme value APIs.
pub fn project_workbench(
    runtime: &GerbilRuntime,
    export: &RootedSchemeObject<'_>,
    session: &UiNativeSession<'_>,
) -> Result<UiWorkbench, UiProjectionError> {
    decode_workbench(runtime.call_rooted(export, &session.root)?)
}

/// Dispatch a typed event and retain a fresh successor without consuming the old state.
pub fn dispatch_event<'runtime>(
    runtime: &'runtime GerbilRuntime,
    export: &RootedSchemeObject<'runtime>,
    session: &UiNativeSession<'runtime>,
    event: UiEvent,
) -> Result<(UiNativeSession<'runtime>, UiIntent), UiProjectionError> {
    dispatch_event_with(runtime, session, event, |request| {
        Ok(runtime.call_rooted(export, request)?)
    })
}

fn dispatch_event_with<'runtime>(
    runtime: &'runtime GerbilRuntime,
    session: &UiNativeSession<'runtime>,
    event: UiEvent,
    call: impl FnOnce(
        &RootedSchemeObject<'_>,
    ) -> Result<RootedSchemeObject<'runtime>, UiProjectionError>,
) -> Result<(UiNativeSession<'runtime>, UiIntent), UiProjectionError> {
    let name = match event {
        UiEvent::NextFocus => "next-focus",
        UiEvent::PreviousFocus => "previous-focus",
        UiEvent::Submit => "submit",
        UiEvent::Dismiss => "dismiss",
        UiEvent::ScrollUp => "scroll-up",
        UiEvent::ScrollDown => "scroll-down",
    };
    let symbol = runtime.intern_identifier(IdentifierKind::Symbol, name)?;
    let request =
        runtime.collection_from_objects(CollectionKind::Vector, &[&session.root, &symbol])?;
    let result = vector(call(&request.into_object())?, 2, 2)?;
    let mut roots = RootedObjectBatch::default();
    result.copy_into(&mut roots)?;
    let next = roots.root_element(0)?;
    let intent = decode_intent(roots.root_element(1)?)?;
    Ok((UiNativeSession { root: next }, intent))
}
