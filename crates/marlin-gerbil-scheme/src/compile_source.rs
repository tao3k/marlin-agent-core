//! CLI adapter for compiling Gerbil source files into typed `marlin` artifacts.

use crate::{
    GerbilArtifactKind, GerbilCommandCompiler, GerbilSource, MARLIN_GERBIL_GXI_ENV,
    default_gerbil_gxi_program,
};
use marlin_agent_runtime::TokioAgentRuntime;
use std::{
    env,
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    process::ExitCode,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::runtime::Builder;

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
    let artifact = Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()?
        .block_on(compile_source_request(&request))?;
    println!("{}", serde_json::to_string_pretty(&artifact)?);
    Ok(())
}

async fn compile_source_request(
    request: &CompileSourceRequest,
) -> Result<crate::GerbilCompiledArtifact, Box<dyn Error>> {
    let source = fs::read_to_string(&request.source_path)?;
    let gxi = default_gerbil_gxi_program();
    if !gxi.exists() {
        return Err(format!(
            "missing gxi executable at {}; set {MARLIN_GERBIL_GXI_ENV} to override",
            gxi.display()
        )
        .into());
    }

    let runtime_root = RuntimeRoot::new("compile-source");
    let compiler = GerbilCommandCompiler::from_default_marlin_runtime_module(runtime_root.path())?;
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = runtime.context();
    compiler
        .compile_with_runtime(
            &context,
            GerbilSource::new(request.source_path.display().to_string(), source),
            request.kind,
        )
        .await
        .map_err(io::Error::other)
        .map_err(Into::into)
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

struct RuntimeRoot {
    path: PathBuf,
}

impl RuntimeRoot {
    fn new(name: &str) -> Self {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        Self {
            path: env::temp_dir().join(format!(
                "marlin-gerbil-scheme-{name}-{}-{suffix}",
                std::process::id()
            )),
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for RuntimeRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
