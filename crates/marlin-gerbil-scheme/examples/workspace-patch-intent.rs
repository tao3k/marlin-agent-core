use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCommandCompiler, GerbilCompiledArtifact, GerbilCompiler,
    GerbilSource, MARLIN_GERBIL_GXI_ENV, default_gerbil_gxi_program, write_gerbil_runtime_assets,
};
use std::{
    error::Error,
    fs, io,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const WORKSPACE_PATCH_INTENT_SOURCE: &str = r#"(workspace-patch-intent "intent:memory"
  (dry-run-first #t)
  (patch
    (reason "gerbil intent")
    (source-agent "gerbil")
    (set-todo "memory.org:1:goal" DONE)
    (set-property "memory.org:1:goal" OWNER "gerbil")
    (mark-memory-candidate "memory.org:1:goal" "long-term")))"#;

fn main() -> Result<(), Box<dyn Error>> {
    let gxi = default_gerbil_gxi_program();
    if !gxi.exists() {
        return Err(format!(
            "missing gxi executable at {}; set {MARLIN_GERBIL_GXI_ENV} to override",
            gxi.display()
        )
        .into());
    }

    let runtime_root = runtime_root();
    write_gerbil_runtime_assets(&runtime_root)?;
    let compiler = GerbilCommandCompiler::from_marlin_runtime_module(gxi, runtime_root.clone());

    let artifact = compiler
        .compile(
            GerbilSource::new(
                "examples/workspace-patch-intent",
                WORKSPACE_PATCH_INTENT_SOURCE,
            ),
            GerbilArtifactKind::WorkspacePatchIntent,
        )
        .map_err(io::Error::other)?;

    let GerbilCompiledArtifact::WorkspacePatchIntent(intent) = artifact else {
        return Err("expected WorkspacePatchIntent artifact".into());
    };

    println!("compiled {}", intent.intent_id);
    println!("dry_run_first {}", intent.dry_run_first);
    println!("ops {}", intent.patch.ops.len());

    let _ = fs::remove_dir_all(runtime_root);
    Ok(())
}

fn runtime_root() -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "marlin-gerbil-scheme-example-{}-{suffix}",
        std::process::id()
    ))
}
