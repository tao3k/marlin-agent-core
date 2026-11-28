// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/engineering/defense/key-rotation-crypto-shredding/KeyRotationCryptoShredding"
// @countermodel marlin-b16-stale-write step=stepMarlinStaleWrite max-steps=6 itf="h.staleWriteViolations > 0"
// @countermodel marlin-b16-shredded-use step=stepMarlinShreddedUse max-steps=6 itf="h.shredReadViolations > 0"
// @countermodel marlin-b16-mutated-rewrap step=stepMarlinMutatedRewrap max-steps=6 itf="h.rewrapMutationViolations > 0"

// Marlin B16 Zero-Downtime Key Rotation & Cryptographic Shredding overlay
// composing KeyRotationCryptoShredding with marlin-agent-kernel's
// KeyRotationShreddingGate (monotonic epoch progression, stale write fence,
// irreversible shred tombstones, and plaintext digest continuity).
module marlin_backend_key_rotation_overlay {
  type State = {
    tenantAadBound: bool,
    tombstoneCommitted: bool
  }

  action init = h' = {
    tenantAadBound: true,
    tombstoneCommitted: false
  }

  action recordShredTombstone =
    if (h.epoch1Shredded and not(h.tombstoneCommitted))
      h' = h.with("tombstoneCommitted", true)
    else
      h' = h

  action commitMarlinCryptoOpSafe =
    if (h.safeOps < 2 and h.tenantAadBound and h.writeEpoch == h.activeEpoch and not(h.writeEpoch == 1 and h.epoch1Shredded) and h.digestPreserved)
      h' = h.with("safeOps", h.safeOps + 1)
    else
      h' = h

  action step = any {
    rotateToEpoch2,
    selectEpoch2Write,
    shredEpoch1Key,
    recordShredTombstone,
    verifyPlaintextDigest,
    commitMarlinCryptoOpSafe
  }

  val safety = (
    h.staleWriteViolations == 0 and
    h.shredReadViolations == 0 and
    h.rewrapMutationViolations == 0
  )

  action commitMarlinStaleWrite =
    if (h.safeOps < 2 and h.writeEpoch != h.activeEpoch and not(h.writeEpoch == 1 and h.epoch1Shredded) and h.digestPreserved)
      h' = h
        .with("safeOps", h.safeOps + 1)
        .with("staleWriteViolations", h.staleWriteViolations + 1)
    else
      h' = h

  action stepMarlinStaleWrite = any { step, commitMarlinStaleWrite }

  action commitMarlinShreddedUse =
    if (h.safeOps < 2 and h.writeEpoch == h.activeEpoch and (h.writeEpoch == 1 and h.epoch1Shredded) and h.digestPreserved)
      h' = h
        .with("safeOps", h.safeOps + 1)
        .with("shredReadViolations", h.shredReadViolations + 1)
    else
      h' = h

  action stepMarlinShreddedUse = any { step, commitMarlinShreddedUse }

  action commitMarlinMutatedRewrap =
    if (h.safeOps < 2 and h.writeEpoch == h.activeEpoch and not(h.writeEpoch == 1 and h.epoch1Shredded) and not(h.digestPreserved))
      h' = h
        .with("safeOps", h.safeOps + 1)
        .with("rewrapMutationViolations", h.rewrapMutationViolations + 1)
    else
      h' = h

  action stepMarlinMutatedRewrap = any { step, commitMarlinMutatedRewrap }
}
