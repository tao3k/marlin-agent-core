pub(super) use super::support::{MARLIN_REQUIRE_REAL_GXI_ENV, local_gxi};
use marlin_gerbil_scheme::GerbilCompiledArtifact;

pub(super) fn assert_release_topology_artifact(artifact: GerbilCompiledArtifact) {
    super::support::assert_release_topology_artifact(artifact);
}

mod artifacts;
mod examples;
mod workflow;
