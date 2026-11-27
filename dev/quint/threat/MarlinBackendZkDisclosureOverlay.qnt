// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/engineering/defense/zk-selective-disclosure/ZkSelectiveDisclosure"
// @countermodel marlin-b18-over-disclosure step=stepMarlinOverDisclosure max-steps=6 itf="h.overDisclosureViolations > 0"
// @countermodel marlin-b18-forged-sigma step=stepMarlinForgedSigma max-steps=6 itf="h.forgedSigmaViolations > 0"
// @countermodel marlin-b18-nullifier-replay step=stepMarlinNullifierReplay max-steps=6 itf="h.nullifierReplayViolations > 0"

// Marlin B18 Zero-Knowledge Selective-Disclosure Compliance Proofs overlay
// composing ZkSelectiveDisclosure with marlin-agent-kernel's
// ZkSelectiveDisclosureGate (zero secret-field over-disclosure, verified Sigma
// proof bound to fresh verifier challenge, and single-use nullifier burn).
module marlin_backend_zk_disclosure_overlay {
  type State = {
    requiredPredicatesProven: bool,
    receiptPinned: bool
  }

  action init = h' = {
    requiredPredicatesProven: true,
    receiptPinned: false
  }

  action admitMarlinZkDisclosureSafe =
    if (h.verifiedDisclosures < 2 and h.requiredPredicatesProven and not(h.secretFieldExposed) and h.sigmaProofValid and h.nullifierFresh)
      h' = h
        .with("verifiedDisclosures", h.verifiedDisclosures + 1)
        .with("nullifierFresh", false)
        .with("receiptPinned", true)
    else
      h' = h

  action step = any {
    exposeSecretField,
    redactSecretField,
    verifySigmaProof,
    issueFreshNullifier,
    admitMarlinZkDisclosureSafe
  }

  val safety = (
    h.overDisclosureViolations == 0 and
    h.forgedSigmaViolations == 0 and
    h.nullifierReplayViolations == 0
  )

  action admitMarlinOverDisclosed =
    if (h.verifiedDisclosures < 2 and h.secretFieldExposed and h.sigmaProofValid and h.nullifierFresh)
      h' = h
        .with("verifiedDisclosures", h.verifiedDisclosures + 1)
        .with("nullifierFresh", false)
        .with("overDisclosureViolations", h.overDisclosureViolations + 1)
    else
      h' = h

  action stepMarlinOverDisclosure = any { step, admitMarlinOverDisclosed }

  action admitMarlinForgedSigma =
    if (h.verifiedDisclosures < 2 and not(h.secretFieldExposed) and not(h.sigmaProofValid) and h.nullifierFresh)
      h' = h
        .with("verifiedDisclosures", h.verifiedDisclosures + 1)
        .with("nullifierFresh", false)
        .with("forgedSigmaViolations", h.forgedSigmaViolations + 1)
    else
      h' = h

  action stepMarlinForgedSigma = any { step, admitMarlinForgedSigma }

  action admitMarlinNullifierReplay =
    if (h.verifiedDisclosures < 2 and not(h.secretFieldExposed) and h.sigmaProofValid and not(h.nullifierFresh))
      h' = h
        .with("verifiedDisclosures", h.verifiedDisclosures + 1)
        .with("nullifierReplayViolations", h.nullifierReplayViolations + 1)
    else
      h' = h

  action stepMarlinNullifierReplay = any { step, admitMarlinNullifierReplay }
}
