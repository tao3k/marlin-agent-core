import Lake
open Lake DSL

package «marlin-proof» where
  version := v!"0.1.0"

lean_lib MarlinProof where
  roots := #[`MarlinProof]

@[default_target]
lean_lib MarlinProofStorage where
  roots := #[`MarlinProof.Storage.ArtifactPointer]
