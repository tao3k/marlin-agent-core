// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/engineering/defense/post-quantum-crypto-agility/PostQuantumCryptoAgility"
// @countermodel marlin-b17-suite-downgrade step=stepMarlinDowngrade max-steps=6 itf="h.downgradeViolations > 0"
// @countermodel marlin-b17-single-leg-bypass step=stepMarlinSingleLeg max-steps=6 itf="h.singleLegBypassViolations > 0"
// @countermodel marlin-b17-ots-leaf-reuse step=stepMarlinOtsReuse max-steps=6 itf="h.otsReuseViolations > 0"

// Marlin B17 Post-Quantum Hybrid Cryptographic Agility & Anti-Downgrade overlay
// composing PostQuantumCryptoAgility with marlin-agent-kernel's
// PostQuantumAgilityGate (hybrid suite floor, dual AND verification legs,
// and single-use stateful OTS leaf consumption).
module marlin_backend_post_quantum_overlay {
  type State = {
    domainSeparated: bool,
    receiptEmitted: bool
  }

  action init = h' = {
    domainSeparated: true,
    receiptEmitted: false
  }

  action acceptMarlinHybridSafe =
    if (h.verifiedCerts < 2 and h.domainSeparated and h.suiteLevel >= 2 and h.classicalLegValid and h.pqLegValid and h.otsLeafFresh)
      h' = h.with("verifiedCerts", h.verifiedCerts + 1).with("otsLeafFresh", false).with("receiptEmitted", true)
    else
      h' = h

  action step = any {
    attemptSuiteDowngrade,
    restoreHybridSuite,
    verifyClassicalLeg,
    verifyPqLeg,
    allocateFreshOtsLeaf,
    acceptMarlinHybridSafe
  }

  val safety = (
    h.downgradeViolations == 0 and
    h.singleLegBypassViolations == 0 and
    h.otsReuseViolations == 0
  )

  action acceptMarlinDowngraded =
    if (h.verifiedCerts < 2 and h.suiteLevel < 2 and h.classicalLegValid and h.pqLegValid and h.otsLeafFresh)
      h' = h
        .with("verifiedCerts", h.verifiedCerts + 1)
        .with("otsLeafFresh", false)
        .with("downgradeViolations", h.downgradeViolations + 1)
    else
      h' = h

  action stepMarlinDowngrade = any { step, acceptMarlinDowngraded }

  action acceptMarlinSingleLeg =
    if (h.verifiedCerts < 2 and h.suiteLevel >= 2 and not(h.classicalLegValid and h.pqLegValid) and h.otsLeafFresh)
      h' = h
        .with("verifiedCerts", h.verifiedCerts + 1)
        .with("otsLeafFresh", false)
        .with("singleLegBypassViolations", h.singleLegBypassViolations + 1)
    else
      h' = h

  action stepMarlinSingleLeg = any { step, acceptMarlinSingleLeg }

  action acceptMarlinOtsReuse =
    if (h.verifiedCerts < 2 and h.suiteLevel >= 2 and h.classicalLegValid and h.pqLegValid and not(h.otsLeafFresh))
      h' = h
        .with("verifiedCerts", h.verifiedCerts + 1)
        .with("otsReuseViolations", h.otsReuseViolations + 1)
    else
      h' = h

  action stepMarlinOtsReuse = any { step, acceptMarlinOtsReuse }
}
