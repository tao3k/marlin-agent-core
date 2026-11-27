// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/engineering/defense/geo-residency-sovereignty/GeoResidencySovereignty"
// @countermodel marlin-b19-sanctioned-region step=stepMarlinSanctionedRegion max-steps=6 itf="h.sanctionedPlacementViolations > 0"
// @countermodel marlin-b19-inference-escape step=stepMarlinInferenceEscape max-steps=6 itf="h.inferenceEscapeViolations > 0"
// @countermodel marlin-b19-unpermitted-egress step=stepMarlinUnpermittedEgress max-steps=6 itf="h.unpermittedEgressViolations > 0"

// Marlin B19 Cross-Region Sovereign Data-Residency & Geo-Fencing overlay
// composing GeoResidencySovereignty with marlin-agent-kernel's
// GeoResidencySovereigntyGate (storage residency, LLM inference confinement,
// anti-P4 intra-region sufficiency, and cross-border permit verification).
module marlin_backend_geo_residency_overlay {
  type State = {
    authorityActive: bool,
    receiptPinned: bool
  }

  action init = h' = {
    authorityActive: true,
    receiptPinned: false
  }

  action admitMarlinGeoSafe =
    if (h.admittedPlacements < 2 and h.authorityActive and h.targetRegionAllowed and h.inferenceRegionAllowed and (not(h.crossBorderTransfer) or h.transferPermitValid))
      h' = h.with("admittedPlacements", h.admittedPlacements + 1).with("receiptPinned", true)
    else
      h' = h

  action step = any {
    allowTargetRegion,
    allowInferenceRegion,
    markCrossBorderTransfer,
    issueCrossBorderPermit,
    admitMarlinGeoSafe
  }

  val safety = (
    h.sanctionedPlacementViolations == 0 and
    h.inferenceEscapeViolations == 0 and
    h.unpermittedEgressViolations == 0
  )

  action admitMarlinSanctioned =
    if (h.admittedPlacements < 2 and not(h.targetRegionAllowed) and h.inferenceRegionAllowed and (not(h.crossBorderTransfer) or h.transferPermitValid))
      h' = h
        .with("admittedPlacements", h.admittedPlacements + 1)
        .with("sanctionedPlacementViolations", h.sanctionedPlacementViolations + 1)
    else
      h' = h

  action stepMarlinSanctionedRegion = any { step, admitMarlinSanctioned }

  action admitMarlinInfEscape =
    if (h.admittedPlacements < 2 and h.targetRegionAllowed and not(h.inferenceRegionAllowed) and (not(h.crossBorderTransfer) or h.transferPermitValid))
      h' = h
        .with("admittedPlacements", h.admittedPlacements + 1)
        .with("inferenceEscapeViolations", h.inferenceEscapeViolations + 1)
    else
      h' = h

  action stepMarlinInferenceEscape = any { step, admitMarlinInfEscape }

  action admitMarlinUnpermitted =
    if (h.admittedPlacements < 2 and h.targetRegionAllowed and h.inferenceRegionAllowed and h.crossBorderTransfer and not(h.transferPermitValid))
      h' = h
        .with("admittedPlacements", h.admittedPlacements + 1)
        .with("unpermittedEgressViolations", h.unpermittedEgressViolations + 1)
    else
      h' = h

  action stepMarlinUnpermittedEgress = any { step, admitMarlinUnpermitted }
}
