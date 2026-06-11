use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCommandCompiler, GerbilCompiledArtifact, GerbilCompiler,
    GerbilSource, MARLIN_GERBIL_GXI_ENV, default_gerbil_gxi_program,
};
use marlin_org_workflow::{
    GerbilWorkspacePatchIntentDryRunner, gerbil_workspace_patch_receipt_evidence,
};
use std::{
    error::Error,
    fs, io,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn main() -> Result<(), Box<dyn Error>> {
    let gxi = default_gerbil_gxi_program();
    if !gxi.exists() {
        return Err(format!(
            "missing gxi executable at {}; set {MARLIN_GERBIL_GXI_ENV} to override",
            gxi.display()
        )
        .into());
    }

    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("workspace-patch-intent-source.ss");
    let source = fs::read_to_string(&source_path)?;
    let runtime_root = runtime_root();
    let compiler = GerbilCommandCompiler::from_default_marlin_runtime_module(&runtime_root)?;

    let artifact = compiler
        .compile(
            GerbilSource::new(source_path.to_string_lossy(), source),
            GerbilArtifactKind::WorkspacePatchIntent,
        )
        .map_err(io::Error::other)?;

    let GerbilCompiledArtifact::WorkspacePatchIntent(intent) = artifact else {
        return Err("expected WorkspacePatchIntent artifact".into());
    };

    let receipt = GerbilWorkspacePatchIntentDryRunner::dry_run(&intent);
    let evidence = gerbil_workspace_patch_receipt_evidence(&receipt);

    println!("compiled {}", intent.intent_id);
    println!("dry_run_first {}", intent.dry_run_first);
    println!("workflow_accepted {}", receipt.validation.accepted);
    println!("evidence_present {}", evidence.present);
    println!("memory_dispatch {}", receipt.memory_dispatch.len());

    let _ = fs::remove_dir_all(runtime_root);
    Ok(())
}

fn runtime_root() -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "marlin-gerbil-scheme-workflow-example-{}-{suffix}",
        std::process::id()
    ))
}
