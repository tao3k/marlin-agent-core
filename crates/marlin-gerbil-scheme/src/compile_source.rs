//! CLI adapter for requesting Gerbil native ABI typed artifact projection.

use crate::GerbilArtifactKind;
use std::{env, error::Error, io, path::PathBuf, process::ExitCode};

const ARTIFACT_KIND_USAGE: &str = "loop-graph, workspace-schema, workspace-view-policy, \
workspace-validation-policy, memory-dispatch-policy, workspace-patch-intent, \
agent-scenario-contract, release-topology";

/// Runs the `compile-source` command-line adapter.
pub fn run_compile_source_cli() -> ExitCode {
    match run_compile_source_from_args(env::args()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run_compile_source_from_args(
    args: impl IntoIterator<Item = String>,
) -> Result<(), Box<dyn Error>> {
    let request = CompileSourceRequest::parse(args.into_iter())?;
    let artifact = compile_source_request(&request)?;
    println!("{}", serde_json::to_string_pretty(&artifact)?);
    Ok(())
}

fn compile_source_request(
    request: &CompileSourceRequest,
) -> Result<crate::GerbilCompiledArtifact, Box<dyn Error>> {
    Err(format!(
        "marlin-gerbil-compile-source is waiting on the native ABI typed projection; \
         Rust must not parse Gerbil source text. Build a \
         GerbilSchemeNativeProjectionRequest for {} as {:?}, then route \
         Gerbil-built Scheme types -> native ABI -> Rust types with a \
         GerbilSchemeNativeProjectionReceipt.",
        request.source_path.display(),
        request.kind
    )
    .into())
}

struct CompileSourceRequest {
    kind: GerbilArtifactKind,
    source_path: PathBuf,
}

impl CompileSourceRequest {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, Box<dyn Error>> {
        let program = args
            .next()
            .unwrap_or_else(|| "marlin-gerbil-compile-source".to_string());
        let kind_arg = match args.next() {
            Some(kind) => kind,
            None => return Err(usage(&program).into()),
        };
        let source_arg = match args.next() {
            Some(source) => source,
            None => return Err(usage(&program).into()),
        };
        if let Some(extra) = args.next() {
            return Err(
                format!("unexpected extra argument {extra:?}\n\n{}", usage(&program)).into(),
            );
        }

        let kind = parse_artifact_kind(&kind_arg)
            .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
        let source_path = PathBuf::from(source_arg);
        Ok(Self { kind, source_path })
    }
}

fn usage(program: &str) -> String {
    format!("usage: {program} <artifact-kind> <source.ss>\n\nartifact kinds: {ARTIFACT_KIND_USAGE}")
}

fn parse_artifact_kind(value: &str) -> Result<GerbilArtifactKind, String> {
    match normalized_artifact_kind(value).as_str() {
        "loopgraph" => Ok(GerbilArtifactKind::LoopGraph),
        "workspaceschema" => Ok(GerbilArtifactKind::WorkspaceSchema),
        "workspaceviewpolicy" => Ok(GerbilArtifactKind::WorkspaceViewPolicy),
        "workspacevalidationpolicy" => Ok(GerbilArtifactKind::WorkspaceValidationPolicy),
        "memorydispatchpolicy" => Ok(GerbilArtifactKind::MemoryDispatchPolicy),
        "workspacepatchintent" => Ok(GerbilArtifactKind::WorkspacePatchIntent),
        "agentscenariocontract" => Ok(GerbilArtifactKind::AgentScenarioContract),
        "releasetopology" => Ok(GerbilArtifactKind::ReleaseTopology),
        _ => Err(format!(
            "unknown artifact kind {value:?}; expected one of: {ARTIFACT_KIND_USAGE}"
        )),
    }
}

fn normalized_artifact_kind(value: &str) -> String {
    value
        .chars()
        .filter(|character| !matches!(character, '-' | '_') && !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}
