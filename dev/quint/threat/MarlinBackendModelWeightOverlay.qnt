// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/engineering/defense/model-weight-inference-attestation/ModelWeightInferenceAttestation"
// @countermodel marlin-b21-unpinned-shard-root step=stepMarlinUnpinnedShardRoot max-steps=6 itf="h.unpinnedWeightViolations > 0"
// @countermodel marlin-b21-stealth-lora-adapter step=stepMarlinStealthLora max-steps=6 itf="h.stealthLoraViolations > 0"
// @countermodel marlin-b21-kernel-or-seed-drift step=stepMarlinKernelOrSeedDrift max-steps=6 itf="h.nondetSamplingViolations > 0"

// Marlin B21 Deterministic Model-Weight Pinning & Reproducible Inference Attestation
// overlay composing ModelWeightInferenceAttestation with marlin-agent-kernel's
// ModelWeightAttestationGate (tensor shard Merkle root, LoRA allowlist, serving kernel
// digest, deterministic temperature/seed lock, and model lineage revocation).
module marlin_backend_model_weight_overlay {
  type State = {
    kernelDigestMatched: bool,
    seedPinned: bool,
    policyRegistered: bool
  }

  action init = h' = {
    kernelDigestMatched: false,
    seedPinned: false,
    policyRegistered: false
  }

  // Register pinned ModelWeightGovernancePolicy (base shard Merkle root + LoRA allowlist + serving kernel).
  action registerMarlinModelPolicy =
    if (not(h.policyRegistered))
      h' = h
        .with("policyRegistered", true)
        .with("weightRootPinned", true)
        .with("adaptersAllowlisted", true)
        .with("kernelDigestMatched", true)
    else
      h' = h

  // Lock greedy temperature=0 and explicit PRNG seed for reproducible inference.
  action lockMarlinGreedySeed =
    if (h.policyRegistered and not(h.seedPinned))
      h' = h
        .with("seedPinned", true)
        .with("samplingDeterministic", true)
    else
      h' = h

  // Admit an inference claim ONLY when policy, shard Merkle root, LoRA allowlist,
  // serving kernel digest, greedy seed lock, and non-revoked lineage all hold.
  action admitMarlinPinnedInference =
    if (h.admittedInferences < 2 and h.policyRegistered and h.weightRootPinned and h.adaptersAllowlisted and h.kernelDigestMatched and h.seedPinned and h.samplingDeterministic and not(h.modelRevoked))
      h' = h.with("admittedInferences", h.admittedInferences + 1)
    else
      h' = h

  action step = any {
    registerMarlinModelPolicy,
    lockMarlinGreedySeed,
    revokeModel,
    admitMarlinPinnedInference
  }

  val safety = (
    h.unpinnedWeightViolations == 0 and
    h.stealthLoraViolations == 0 and
    h.nondetSamplingViolations == 0
  )

  // Countermodel: bypassing base-weight shard Merkle root check admits unpinned/swapped weights.
  action admitMarlinUnpinnedShard =
    if (h.admittedInferences < 2 and not(h.weightRootPinned))
      h' = h
        .with("admittedInferences", h.admittedInferences + 1)
        .with("unpinnedWeightViolations", h.unpinnedWeightViolations + 1)
    else
      h' = h

  action stepMarlinUnpinnedShardRoot = any { step, admitMarlinUnpinnedShard }

  // Countermodel: bypassing LoRA allowlist admits a stealth/unpinned adapter.
  action admitMarlinStealthLora =
    if (h.admittedInferences < 2 and not(h.adaptersAllowlisted))
      h' = h
        .with("admittedInferences", h.admittedInferences + 1)
        .with("stealthLoraViolations", h.stealthLoraViolations + 1)
    else
      h' = h

  action stepMarlinStealthLora = any { step, admitMarlinStealthLora }

  // Countermodel: bypassing kernel digest / deterministic seed / revocation check admits drifted inference.
  action admitMarlinKernelOrSeedDrift =
    if (h.admittedInferences < 2 and (not(h.kernelDigestMatched) or not(h.seedPinned) or h.modelRevoked))
      h' = h
        .with("admittedInferences", h.admittedInferences + 1)
        .with("nondetSamplingViolations", h.nondetSamplingViolations + 1)
    else
      h' = h

  action stepMarlinKernelOrSeedDrift = any { step, admitMarlinKernelOrSeedDrift }
}
