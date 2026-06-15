use marlin_gerbil_scheme::GerbilCompiledArtifact;
use serde_json::json;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let artifact = workspace_patch_intent_artifact()?;
    let GerbilCompiledArtifact::WorkspacePatchIntent(intent) = artifact else {
        return Err("expected WorkspacePatchIntent artifact".into());
    };

    println!("compiled {}", intent.intent_id);
    println!("dry_run_first {}", intent.dry_run_first);
    println!("ops {}", intent.patch.ops.len());

    Ok(())
}

fn workspace_patch_intent_artifact() -> Result<GerbilCompiledArtifact, serde_json::Error> {
    serde_json::from_value(json!({
        "WorkspacePatchIntent": {
            "intent_id": "intent:memory",
            "patch": {
                "reason": "gerbil intent",
                "source_agent": "gerbil",
                "ops": [
                    {"SetTodo": {"node": "memory.org:1:goal", "state": "Done"}},
                    {"SetProperty": {"node": "memory.org:1:goal", "key": "OWNER", "value": "gerbil"}},
                    {"MarkMemoryCandidate": {"node": "memory.org:1:goal", "dispatch": "long-term"}}
                ]
            },
            "dry_run_first": true
        }
    }))
}
