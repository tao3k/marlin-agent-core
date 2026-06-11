use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCommandCompiler, GerbilCompiler, GerbilSource, MARLIN_GERBIL_GXI_ENV,
    default_gerbil_gxi_program,
};
use std::{
    env,
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const ARTIFACT_KIND_USAGE: &str = "loop-graph, workspace-schema, workspace-view-policy, \
workspace-validation-policy, memory-dispatch-policy, workspace-patch-intent, \
agent-scenario-contract, release-topology";

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "compile-source".to_string());
    let kind_arg = match args.next() {
        Some(kind) => kind,
        None => return Err(usage(&program).into()),
    };
    let source_arg = match args.next() {
        Some(source) => source,
        None => return Err(usage(&program).into()),
    };
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument {extra:?}\n\n{}", usage(&program)).into());
    }

    let kind = parse_artifact_kind(&kind_arg)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let source_path = PathBuf::from(source_arg);
    let source = fs::read_to_string(&source_path)?;

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
    let artifact = compiler
        .compile(
            GerbilSource::new(source_path.display().to_string(), source),
            kind,
        )
        .map_err(io::Error::other)?;

    println!("{}", serde_json::to_string_pretty(&artifact)?);
    Ok(())
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
