// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/engineering/defense/supply-chain-provenance/SupplyChainProvenance"
// @countermodel marlin-b14-slopsquatting step=stepMarlinSlopsquatting max-steps=6 itf="h.slopsquatViolations > 0"
// @countermodel marlin-b14-cve-tombstone step=stepMarlinCveTombstone max-steps=6 itf="h.cveViolations > 0"
// @countermodel marlin-b14-non-reproducible step=stepMarlinNonReproducible max-steps=6 itf="h.reproViolations > 0"

// Marlin B14 Hermetic Supply-Chain Provenance & Anti-Slopsquatting overlay
// composing SupplyChainProvenance with marlin-agent-kernel's
// SupplyChainProvenanceGate (lockfile pin, CVE tombstone, hermetic sandbox,
// and 2-builder reproducibility parity).
module marlin_backend_supply_chain_overlay {
  type State = {
    signedRegistryLoaded: bool,
    receiptPinned: bool
  }

  action init = h' = {
    signedRegistryLoaded: true,
    receiptPinned: false
  }

  action admitMarlinReleaseSafe =
    if (h.admittedBuilds < 2 and h.signedRegistryLoaded and h.lockfilePinned and not(h.cveTombstoned) and h.hermeticSandbox and h.twoBuilderMatch)
      h' = h.with("admittedBuilds", h.admittedBuilds + 1).with("receiptPinned", true)
    else
      h' = h

  action step = any {
    pinLockfileDeps,
    flagCveTombstone,
    enableHermeticSandbox,
    verifySecondBuilderParity,
    admitMarlinReleaseSafe
  }

  val safety = (
    h.slopsquatViolations == 0 and
    h.cveViolations == 0 and
    h.reproViolations == 0
  )

  action admitMarlinSlopsquat =
    if (h.admittedBuilds < 2 and not(h.lockfilePinned) and not(h.cveTombstoned) and h.hermeticSandbox and h.twoBuilderMatch)
      h' = h
        .with("admittedBuilds", h.admittedBuilds + 1)
        .with("slopsquatViolations", h.slopsquatViolations + 1)
    else
      h' = h

  action stepMarlinSlopsquatting = any { step, admitMarlinSlopsquat }

  action admitMarlinCve =
    if (h.admittedBuilds < 2 and h.lockfilePinned and h.cveTombstoned and h.hermeticSandbox and h.twoBuilderMatch)
      h' = h
        .with("admittedBuilds", h.admittedBuilds + 1)
        .with("cveViolations", h.cveViolations + 1)
    else
      h' = h

  action stepMarlinCveTombstone = any { step, admitMarlinCve }

  action admitMarlinNonRepro =
    if (h.admittedBuilds < 2 and h.lockfilePinned and not(h.cveTombstoned) and not(h.hermeticSandbox and h.twoBuilderMatch))
      h' = h
        .with("admittedBuilds", h.admittedBuilds + 1)
        .with("reproViolations", h.reproViolations + 1)
    else
      h' = h

  action stepMarlinNonReproducible = any { step, admitMarlinNonRepro }
}
