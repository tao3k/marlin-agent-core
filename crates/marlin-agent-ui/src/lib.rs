// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed projection of the selected Scheme UI workbench through the SDK.
//!
//! The caller owns VM initialization and startup package selection. The SDK's
//! standard module loader resolves the selected publication and procedures.

use std::collections::HashSet;

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, LinkedObjectExport,
    NativeError, ObjectKind, RootedCallError, RootedObjectBatch, RootedSchemeCollection,
    RootedSchemeObject, SchemeScalar,
};

const MAX_REGIONS: usize = 20;
const MAX_STATUS: usize = 8;
const MAX_EXTENSIONS: usize = 16;
const MAX_METHOD_BUNDLES: usize = 16;
const MAX_COMMANDS: usize = 32;
const MAX_TEXT_CHARS: usize = 256;
const MAX_ID_BYTES: usize = 1024;

/// A UI component's domain identity, not a Scheme root or object address.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UiComponentId(String);

/// A displayable command name; this value does not authorize a handler.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UiCommandName(String);

impl UiCommandName {
    /// Check a partial command without scanning past the admitted name limit.
    pub fn is_valid_prefix(value: &str) -> bool {
        value.len() <= 32
            && value.starts_with('/')
            && value.as_bytes()[1..]
                .iter()
                .all(|byte| byte.is_ascii_lowercase() || *byte == b'-')
    }

    pub fn new(value: &str) -> Result<Self, UiProjectionError> {
        if value.len() < 2 || !Self::is_valid_prefix(value) {
            return Err(UiProjectionError::Invalid("invalid command name"));
        }
        Ok(Self(value.to_owned()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiCommand {
    pub name: UiCommandName,
    pub description: String,
    pub accepts_arguments: bool,
}

impl UiComponentId {
    /// Construct a bounded, nonempty domain identity.
    pub fn new(value: &str) -> Result<Self, UiProjectionError> {
        if value.is_empty() || value.len() > MAX_ID_BYTES {
            return Err(UiProjectionError::Invalid("invalid identity length"));
        }
        Ok(Self(value.to_owned()))
    }
    /// The identity's spelling. Identity admission rejects duplicate spellings.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One declarative workbench region, without authority to execute effects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiRegion {
    Transcript,
    Composer,
    Commands(Vec<UiCommand>),
    Status(Vec<UiStatusItem>),
    Extension {
        id: UiComponentId,
        label: String,
        focusable: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiStatusItem {
    pub id: UiComponentId,
    pub label: String,
}

/// Host display facts and already-rooted Scheme extension contributions.
/// The selected publication and extension provenance remain the VM/module
/// owner's responsibility.
pub struct UiOpenFacts<'a, 'runtime> {
    pub model_label: &'a str,
    pub session_label: &'a str,
    pub status: &'a [UiStatusItem],
    pub extensions: &'a [RootedSchemeObject<'runtime>],
    pub method_bundles: &'a [RootedSchemeObject<'runtime>],
}

/// An owned candidate. Publication and effect admission remain host duties.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiWorkbench {
    pub model_label: String,
    pub session_label: String,
    pub regions: Vec<UiRegion>,
    pub focus: UiComponentId,
}

impl UiWorkbench {
    pub fn commands(&self) -> &[UiCommand] {
        self.regions
            .iter()
            .find_map(|region| match region {
                UiRegion::Commands(commands) => Some(commands.as_slice()),
                _ => None,
            })
            .unwrap_or(&[])
    }
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiEvent {
    NextFocus,
    PreviousFocus,
    Submit,
    Dismiss,
    ScrollUp,
    ScrollDown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiBrowseDirection {
    Next,
    Previous,
}

/// A presentation proposal; only the runtime can authorize its effects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiIntent {
    SubmitDraft,
    Focus(UiComponentId),
    Scroll(UiScrollDirection),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiScrollDirection {
    Up,
    Down,
}

#[derive(Debug)]
pub enum UiProjectionError {
    Native(NativeError),
    Call(RootedCallError),
    Invalid(&'static str),
}
impl std::fmt::Display for UiProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native(error) => write!(formatter, "{error}"),
            Self::Call(error) => write!(formatter, "{error}"),
            Self::Invalid(reason) => write!(formatter, "invalid UI projection: {reason}"),
        }
    }
}
impl std::error::Error for UiProjectionError {}
impl From<RootedCallError> for UiProjectionError {
    fn from(error: RootedCallError) -> Self {
        Self::Call(error)
    }
}
impl From<NativeError> for UiProjectionError {
    fn from(error: NativeError) -> Self {
        Self::Native(error)
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

/// Convert a rooted Scheme intent; this does not authorize the action.
pub fn decode_intent(value: RootedSchemeObject<'_>) -> Result<UiIntent, UiProjectionError> {
    let items = vector(value, 1, 2)?;
    let mut names = IdentifierBuffer::default();
    let kind = identifier(items.element(0)?, &mut names)?;
    match (kind.as_str(), items.len()) {
        ("submit-draft", 1) => Ok(UiIntent::SubmitDraft),
        ("focus", 2) => Ok(UiIntent::Focus(identifier(items.element(1)?, &mut names)?)),
        ("scroll", 2) => match identifier(items.element(1)?, &mut names)?.as_str() {
            "up" => Ok(UiIntent::Scroll(UiScrollDirection::Up)),
            "down" => Ok(UiIntent::Scroll(UiScrollDirection::Down)),
            _ => Err(UiProjectionError::Invalid("invalid scroll direction")),
        },
        _ => Err(UiProjectionError::Invalid("invalid UI intent")),
    }
}

/// Validate and copy one Scheme UI projection into independent Rust values.
pub fn decode_workbench(value: RootedSchemeObject<'_>) -> Result<UiWorkbench, UiProjectionError> {
    let root = vector(value, 4, 4)?;
    let model = string(root.element(0)?)?;
    let session = string(root.element(1)?)?;
    let regions = vector(root.element(2)?, 4, MAX_REGIONS)?;
    let mut names = IdentifierBuffer::default();
    let focus = identifier(root.element(3)?, &mut names)?;
    let mut ids = HashSet::new();
    let mut result = Vec::with_capacity(regions.len());
    for index in 0..regions.len() {
        let region = vector(regions.element(index)?, 3, 3)?;
        let id = identifier(region.element(0)?, &mut names)?;
        let kind = identifier(region.element(1)?, &mut names)?;
        if !ids.insert(id.0.clone()) {
            return Err(UiProjectionError::Invalid("duplicate component identity"));
        }
        let payload = region.element(2)?;
        let node = match kind.as_str() {
            "transcript" if index == 0 && id.as_str() == "transcript" => {
                vector(payload, 0, 0)?;
                UiRegion::Transcript
            }
            "composer" if index == 1 && id.as_str() == "composer" => {
                vector(payload, 0, 0)?;
                UiRegion::Composer
            }
            "commands" if index == 2 && id.as_str() == "commands" => {
                let entries = vector(payload, 0, MAX_COMMANDS)?;
                let mut commands = Vec::with_capacity(entries.len());
                let mut names = HashSet::with_capacity(entries.len());
                for i in 0..entries.len() {
                    let row = vector(entries.element(i)?, 3, 3)?;
                    let name = UiCommandName::new(&string(row.element(0)?)?)?;
                    if !names.insert(name.clone()) {
                        return Err(UiProjectionError::Invalid("duplicate command name"));
                    }
                    let description = string(row.element(1)?)?;
                    let accepts_arguments = match row.element(2)?.as_ref().scalar()? {
                        SchemeScalar::Boolean(value) => value,
                        _ => {
                            return Err(UiProjectionError::Invalid(
                                "invalid command arguments flag",
                            ));
                        }
                    };
                    commands.push(UiCommand {
                        name,
                        description,
                        accepts_arguments,
                    });
                }
                UiRegion::Commands(commands)
            }
            "status" if index + 1 == regions.len() && id.as_str() == "status" => {
                let children = vector(payload, 0, MAX_STATUS)?;
                let mut items = Vec::with_capacity(children.len());
                for item_index in 0..children.len() {
                    let item = vector(children.element(item_index)?, 3, 3)?;
                    let id = identifier(item.element(0)?, &mut names)?;
                    if identifier(item.element(1)?, &mut names)?.as_str() != "status-item"
                        || !ids.insert(id.0.clone())
                    {
                        return Err(UiProjectionError::Invalid(
                            "invalid status identity or kind",
                        ));
                    }
                    items.push(UiStatusItem {
                        id,
                        label: string(item.element(2)?)?,
                    });
                }
                UiRegion::Status(items)
            }
            "extension" if index >= 3 && index + 1 < regions.len() => {
                let data = vector(payload, 2, 2)?;
                let label = string(data.element(0)?)?;
                let focusable = match data.element(1)?.as_ref().scalar()? {
                    SchemeScalar::Boolean(value) => value,
                    _ => return Err(UiProjectionError::Invalid("invalid focusable value")),
                };
                UiRegion::Extension {
                    id,
                    label,
                    focusable,
                }
            }
            _ => return Err(UiProjectionError::Invalid("invalid workbench region")),
        };
        result.push(node);
    }
    if !matches!(result.last(), Some(UiRegion::Status(_))) {
        return Err(UiProjectionError::Invalid("missing status region"));
    }
    let focus_valid = result.iter().any(|region| match region {
        UiRegion::Transcript => focus.as_str() == "transcript",
        UiRegion::Composer => focus.as_str() == "composer",
        UiRegion::Commands(_) => focus.as_str() == "commands",
        UiRegion::Extension {
            id,
            focusable: true,
            ..
        } => *id == focus,
        _ => false,
    });
    if !focus_valid {
        return Err(UiProjectionError::Invalid("invalid focused component"));
    }
    Ok(UiWorkbench {
        model_label: model,
        session_label: session,
        regions: result,
        focus,
    })
}

fn vector(
    value: RootedSchemeObject<'_>,
    min: usize,
    max: usize,
) -> Result<RootedSchemeCollection<'_>, UiProjectionError> {
    let value = value.into_collection(CollectionKind::Vector)?;
    if !(min..=max).contains(&value.len()) {
        return Err(UiProjectionError::Invalid("invalid vector size"));
    }
    Ok(value)
}

fn string(value: RootedSchemeObject<'_>) -> Result<String, UiProjectionError> {
    let value = value.into_string()?;
    if value.len().into_result()? > MAX_TEXT_CHARS {
        return Err(UiProjectionError::Invalid("text exceeds UI limit"));
    }
    Ok(value.to_string().into_result()?)
}

fn identifier(
    value: RootedSchemeObject<'_>,
    buffer: &mut IdentifierBuffer,
) -> Result<UiComponentId, UiProjectionError> {
    if value.as_ref().kind()? != ObjectKind::Symbol {
        return Err(UiProjectionError::Invalid("expected symbol"));
    }
    value.as_ref().copy_identifier_names(buffer)?;
    let Some((IdentifierKind::Symbol, name)) = buffer.get(0) else {
        return Err(UiProjectionError::Invalid("invalid symbol"));
    };
    if buffer.len() != 1 || name.is_empty() || name.len() > MAX_ID_BYTES {
        return Err(UiProjectionError::Invalid("invalid identity length"));
    }
    Ok(UiComponentId(name.to_owned()))
}
