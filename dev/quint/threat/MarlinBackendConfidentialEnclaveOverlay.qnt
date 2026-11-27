// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/engineering/defense/confidential-enclave-attestation/ConfidentialEnclaveAttestation"
// @countermodel marlin-b15-debug-enclave step=stepMarlinDebugEnclave max-steps=6 itf="h.debugLeakViolations > 0"
// @countermodel marlin-b15-unmeasured-enclave step=stepMarlinUnmeasuredEnclave max-steps=6 itf="h.measurementViolations > 0"
// @countermodel marlin-b15-stale-tcb-nonce step=stepMarlinStaleTcbNonce max-steps=6 itf="h.tcbOrNonceViolations > 0"

// Marlin B15 Confidential Compute / Hardware TEE Attestation overlay composing
// ConfidentialEnclaveAttestation with marlin-agent-kernel's ConfidentialEnclaveGate
// (anti-debug lockout, B14-bound measurement pinning, TCB SVN floor, and single-use nonce).
module marlin_backend_confidential_enclave_overlay {
  type State = {
    b14ArtifactBound: bool,
    receiptPinned: bool
  }

  action init = h' = {
    b14ArtifactBound: true,
    receiptPinned: false
  }

  action releaseMarlinEnclaveKeySafe =
    if (h.keysReleased < 2 and h.b14ArtifactBound and h.debugDisabled and h.measurementMatched and h.tcbFloorMet and h.freshNonceBound)
      h' = h.with("keysReleased", h.keysReleased + 1).with("freshNonceBound", false).with("receiptPinned", true)
    else
      h' = h

  action step = any {
    lockDebugMode,
    verifyEnclaveMeasurement,
    verifyTcbSvnFloor,
    bindFreshChallengeNonce,
    releaseMarlinEnclaveKeySafe
  }

  val safety = (
    h.debugLeakViolations == 0 and
    h.measurementViolations == 0 and
    h.tcbOrNonceViolations == 0
  )

  action releaseMarlinKeyDebug =
    if (h.keysReleased < 2 and not(h.debugDisabled) and h.measurementMatched and h.tcbFloorMet and h.freshNonceBound)
      h' = h
        .with("keysReleased", h.keysReleased + 1)
        .with("debugLeakViolations", h.debugLeakViolations + 1)
    else
      h' = h

  action stepMarlinDebugEnclave = any { step, releaseMarlinKeyDebug }

  action releaseMarlinKeyUnmeasured =
    if (h.keysReleased < 2 and h.debugDisabled and not(h.measurementMatched) and h.tcbFloorMet and h.freshNonceBound)
      h' = h
        .with("keysReleased", h.keysReleased + 1)
        .with("measurementViolations", h.measurementViolations + 1)
    else
      h' = h

  action stepMarlinUnmeasuredEnclave = any { step, releaseMarlinKeyUnmeasured }

  action releaseMarlinKeyStaleTcb =
    if (h.keysReleased < 2 and h.debugDisabled and h.measurementMatched and not(h.tcbFloorMet and h.freshNonceBound))
      h' = h
        .with("keysReleased", h.keysReleased + 1)
        .with("tcbOrNonceViolations", h.tcbOrNonceViolations + 1)
    else
      h' = h

  action stepMarlinStaleTcbNonce = any { step, releaseMarlinKeyStaleTcb }
}
