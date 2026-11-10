import Lake
open Lake DSL

package «marlin-proof» where
  version := v!"0.1.0"

require LeanPoo from git
  "https://github.com/tao3k/lean-poo.git" @ "1c12d39eb5e2a8220b48f5884cb5039741e2b536"

lean_lib MarlinProof where
  roots := #[`MarlinProof]

@[default_target]
lean_lib MarlinProofStorage where
  roots := #[`MarlinProof.Storage.ArtifactPointer]
