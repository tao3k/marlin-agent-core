// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/engineering/defense/context-distillation-provenance/ContextDistillationProvenance"
// @countermodel marlin-b12-pinned-amnesia step=stepMarlinPinnedAmnesia max-steps=6 itf="h.amnesiaViolations > 0"
// @countermodel marlin-b12-citation-fabrication step=stepMarlinCitationFabrication max-steps=6 itf="h.citationFabricationViolations > 0"
// @countermodel marlin-b12-secrecy-strip step=stepMarlinSecrecyStrip max-steps=6 itf="h.secrecyStripViolations > 0"

// Marlin B12 Controlled Context Distillation & Epistemic Provenance overlay composing
// ContextDistillationProvenance with marlin-agent-kernel's ContextDistillationGate
// (anti-amnesia M1 pin retention, citation grounding, integrity meet, secrecy union,
// and strict token budget enforcement).
module marlin_backend_context_distillation_overlay {
  type State = {
    citationsGrounded: bool,
    withinTokenBudget: bool,
    citationFabricationViolations: int
  }

  action init = h' = {
    citationsGrounded: false,
    withinTokenBudget: false,
    citationFabricationViolations: 0
  }

  action groundCitationsAndBudget =
    if (not(h.citationsGrounded) or not(h.withinTokenBudget))
      h' = h
        .with("citationsGrounded", true)
        .with("withinTokenBudget", true)
    else
      h' = h

  action commitMarlinDistillationSafe =
    if (h.safeDistillations < 2 and h.citationsGrounded and h.withinTokenBudget and (not(h.hasPinnedConstraint) or h.pinnedRetained) and (not(h.hasUntrustedInput) or h.outputMarkedUntrusted) and (not(h.hasSecretTag) or h.secretPreserved))
      h' = h.with("safeDistillations", h.safeDistillations + 1)
    else
      h' = h

  action step = any {
    ingestPinnedConstraint,
    ingestUntrustedSnippet,
    ingestSecretFact,
    copyPinnedIntoSummary,
    tagSummaryUntrusted,
    propagateSecretTag,
    groundCitationsAndBudget,
    commitMarlinDistillationSafe
  }

  val safety = (
    h.amnesiaViolations == 0 and
    h.launderingViolations == 0 and
    h.secrecyStripViolations == 0 and
    h.citationFabricationViolations == 0
  )

  action commitMarlinPinnedAmnesia =
    if (h.safeDistillations < 2 and h.hasPinnedConstraint and not(h.pinnedRetained))
      h' = h
        .with("safeDistillations", h.safeDistillations + 1)
        .with("amnesiaViolations", h.amnesiaViolations + 1)
    else
      h' = h

  action stepMarlinPinnedAmnesia = any { step, commitMarlinPinnedAmnesia }

  action commitMarlinFabricatedCitation =
    if (h.safeDistillations < 2 and not(h.citationsGrounded))
      h' = h
        .with("safeDistillations", h.safeDistillations + 1)
        .with("citationFabricationViolations", h.citationFabricationViolations + 1)
    else
      h' = h

  action stepMarlinCitationFabrication = any { step, commitMarlinFabricatedCitation }

  action commitMarlinSecrecyStrip =
    if (h.safeDistillations < 2 and h.hasSecretTag and not(h.secretPreserved))
      h' = h
        .with("safeDistillations", h.safeDistillations + 1)
        .with("secrecyStripViolations", h.secrecyStripViolations + 1)
    else
      h' = h

  action stepMarlinSecrecyStrip = any { step, commitMarlinSecrecyStrip }
}
