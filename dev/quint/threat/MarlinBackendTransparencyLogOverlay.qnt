// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/engineering/defense/transparency-log-equivocation/TransparencyLogEquivocation"
// @countermodel marlin-b9-unverified-inclusion step=stepMarlinUnverifiedInclusion max-steps=6 itf="h.unverifiedEntryAccepts > 0"
// @countermodel marlin-b9-history-rewrite step=stepMarlinHistoryRewrite max-steps=6 itf="h.historyRewriteAccepts > 0"
// @countermodel marlin-b9-split-view-fraud step=stepMarlinSplitView max-steps=6 itf="h.equivocationAccepts > 0"

// Marlin B9 Verifiable Transparency Log (RFC 6962) & Split-View Equivocation Defense
// overlay composing TransparencyLogEquivocation with marlin-agent-kernel's
// GovernanceTransparencyLog (inclusion proof, append-only consistency proof, and
// split-view fraud-proof detection).
module marlin_backend_transparency_log_overlay {
  type State = {
    rfc6962DomainSeparated: bool,
    fraudProofEmitted: bool
  }

  action init = h' = {
    rfc6962DomainSeparated: true,
    fraudProofEmitted: false
  }

  action recordSplitViewFraudProof =
    if (h.equivocationDetected and not(h.fraudProofEmitted))
      h' = h.with("fraudProofEmitted", true)
    else
      h' = h

  action acceptMarlinAuditSafe =
    if (h.verifiedAudits < 2 and h.rfc6962DomainSeparated and h.sth2Published and h.inclusionProofValid and h.consistencyProofValid and not(h.equivocationDetected) and not(h.fraudProofEmitted))
      h' = h.with("verifiedAudits", h.verifiedAudits + 1)
    else
      h' = h

  action step = any {
    publishInitialSth,
    verifyEntryInclusion,
    appendConsistentSth2,
    attemptHistoryRewrite,
    emitSplitViewSth,
    recordSplitViewFraudProof,
    acceptMarlinAuditSafe
  }

  val safety = (
    h.unverifiedEntryAccepts == 0 and
    h.historyRewriteAccepts == 0 and
    h.equivocationAccepts == 0
  )

  action acceptMarlinUnverifiedInclusion =
    if (h.verifiedAudits < 2 and h.sth2Published and not(h.inclusionProofValid) and h.consistencyProofValid and not(h.equivocationDetected))
      h' = h
        .with("verifiedAudits", h.verifiedAudits + 1)
        .with("unverifiedEntryAccepts", h.unverifiedEntryAccepts + 1)
    else
      h' = h

  action stepMarlinUnverifiedInclusion = any { step, acceptMarlinUnverifiedInclusion }

  action acceptMarlinHistoryRewrite =
    if (h.verifiedAudits < 2 and h.sth2Published and h.inclusionProofValid and not(h.consistencyProofValid) and not(h.equivocationDetected))
      h' = h
        .with("verifiedAudits", h.verifiedAudits + 1)
        .with("historyRewriteAccepts", h.historyRewriteAccepts + 1)
    else
      h' = h

  action stepMarlinHistoryRewrite = any { step, acceptMarlinHistoryRewrite }

  action acceptMarlinSplitView =
    if (h.verifiedAudits < 2 and h.sth2Published and h.inclusionProofValid and h.consistencyProofValid and h.equivocationDetected)
      h' = h
        .with("verifiedAudits", h.verifiedAudits + 1)
        .with("equivocationAccepts", h.equivocationAccepts + 1)
    else
      h' = h

  action stepMarlinSplitView = any { step, acceptMarlinSplitView }
}
