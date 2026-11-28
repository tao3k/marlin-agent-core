// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/engineering/defense/federated-org-reciprocity/FederatedOrgReciprocity"
// @countermodel marlin-b20-unratified-treaty step=stepMarlinUnratifiedTreaty max-steps=6 itf="h.unratifiedTreatyViolations > 0"
// @countermodel marlin-b20-unilateral-sign step=stepMarlinUnilateralSign max-steps=6 itf="h.unilateralSignViolations > 0"
// @countermodel marlin-b20-dispute-freeze step=stepMarlinDisputeFreeze max-steps=6 itf="h.disputeFreezeViolations > 0"

// Marlin B20 Multi-Org Federated Reciprocity & Dispute Arbitration overlay
// composing FederatedOrgReciprocity with marlin-agent-kernel's
// FederatedOrgReciprocityGate (ratified bilateral treaty, dual org countersigs,
// anti-self-spoofing, and immediate dispute freeze).
module marlin_backend_federated_org_overlay {
  type State = {
    distinctOrgPair: bool,
    receiptPinned: bool
  }

  action init = h' = {
    distinctOrgPair: true,
    receiptPinned: false
  }

  action admitMarlinFederatedOpSafe =
    if (h.admittedFedOps < 2 and h.distinctOrgPair and h.treatyRatified and h.originOrgSigned and h.targetOrgSigned and not(h.disputeFreezeActive))
      h' = h.with("admittedFedOps", h.admittedFedOps + 1).with("receiptPinned", true)
    else
      h' = h

  action step = any {
    ratifyBilateralTreaty,
    originOrgSign,
    targetOrgSign,
    raiseDisputeFreeze,
    resolveDisputeFreeze,
    admitMarlinFederatedOpSafe
  }

  val safety = (
    h.unratifiedTreatyViolations == 0 and
    h.unilateralSignViolations == 0 and
    h.disputeFreezeViolations == 0
  )

  action admitMarlinOpUnratified =
    if (h.admittedFedOps < 2 and not(h.treatyRatified) and h.originOrgSigned and h.targetOrgSigned and not(h.disputeFreezeActive))
      h' = h
        .with("admittedFedOps", h.admittedFedOps + 1)
        .with("unratifiedTreatyViolations", h.unratifiedTreatyViolations + 1)
    else
      h' = h

  action stepMarlinUnratifiedTreaty = any { step, admitMarlinOpUnratified }

  action admitMarlinOpUnilateral =
    if (h.admittedFedOps < 2 and h.treatyRatified and not(h.originOrgSigned and h.targetOrgSigned) and not(h.disputeFreezeActive))
      h' = h
        .with("admittedFedOps", h.admittedFedOps + 1)
        .with("unilateralSignViolations", h.unilateralSignViolations + 1)
    else
      h' = h

  action stepMarlinUnilateralSign = any { step, admitMarlinOpUnilateral }

  action admitMarlinOpFrozen =
    if (h.admittedFedOps < 2 and h.treatyRatified and h.originOrgSigned and h.targetOrgSigned and h.disputeFreezeActive)
      h' = h
        .with("admittedFedOps", h.admittedFedOps + 1)
        .with("disputeFreezeViolations", h.disputeFreezeViolations + 1)
    else
      h' = h

  action stepMarlinDisputeFreeze = any { step, admitMarlinOpFrozen }
}
