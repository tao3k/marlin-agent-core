// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// MAR-SRCH-R05 (Downstream Refinement of upstream
// `engineering.defense.search-evidence-chain-integrity`):
// Verifies Scenario B (Dusa + ASCENT) search evidence-chain accumulation,
// structural vs comment-only drift, DE01 negative-domain expansion,
// KV-cache epoch/pin fencing, M2->M1 poison immunization, and ATMS
// multi-justification survival.
module searchEvidenceChainIntegrity {
  type State = {
    indexEpoch: int,
    commentOnlyEdit: bool,
    structuralDrifted: bool,
    negDomainExpanded: bool,
    oracleVerified: bool,
    rootAdmitted: bool,
    rootEvidenceEpoch: int,
    derivedActive: bool,
    hasAltCleanSupport: bool,
    poisonRetracted: bool,
    kvCachedEpoch: int,
    pinnedRetained: bool,
    staleStructuralReuseViolations: int,
    staleNegDomainReuseViolations: int,
    kvCacheContaminationViolations: int,
    poisonedPromotionViolations: int,
    zombieCascadeViolations: int,
    validEvidenceReuses: int,
    validKvServes: int
  }
  var h: State

  action init = h' = {
    indexEpoch: 1,
    commentOnlyEdit: false,
    structuralDrifted: false,
    negDomainExpanded: false,
    oracleVerified: false,
    rootAdmitted: false,
    rootEvidenceEpoch: 0,
    derivedActive: false,
    hasAltCleanSupport: false,
    poisonRetracted: false,
    kvCachedEpoch: 0,
    pinnedRetained: true,
    staleStructuralReuseViolations: 0,
    staleNegDomainReuseViolations: 0,
    kvCacheContaminationViolations: 0,
    poisonedPromotionViolations: 0,
    zombieCascadeViolations: 0,
    validEvidenceReuses: 0,
    validKvServes: 0
  }

  action verifyWithSearchOracle =
    if (not(h.oracleVerified)) h' = h.with("oracleVerified", true) else h' = h

  action admitM1Evidence =
    if (not(h.rootAdmitted) and h.oracleVerified and not(h.poisonRetracted))
      h' = h
        .with("rootAdmitted", true)
        .with("rootEvidenceEpoch", h.indexEpoch)
        .with("structuralDrifted", false)
        .with("negDomainExpanded", false)
        .with("kvCachedEpoch", h.indexEpoch)
    else h' = h

  action deriveScenarioBChain =
    if (h.rootAdmitted and h.rootEvidenceEpoch == h.indexEpoch and not(h.structuralDrifted) and not(h.derivedActive))
      h' = h.with("derivedActive", true)
    else h' = h

  action attachAltCleanJustification =
    if (h.derivedActive and not(h.hasAltCleanSupport))
      h' = h.with("hasAltCleanSupport", true)
    else h' = h

  action applyCommentOnlyEdit =
    if (h.rootAdmitted and not(h.commentOnlyEdit))
      h' = h.with("commentOnlyEdit", true)
    else h' = h

  action reuseStructuralEvidence =
    if (h.rootAdmitted and h.rootEvidenceEpoch == h.indexEpoch and not(h.structuralDrifted) and not(h.negDomainExpanded))
      h' = h.with("validEvidenceReuses", h.validEvidenceReuses + 1)
    else h' = h

  action applyStructuralAstDrift =
    if (h.rootAdmitted and h.indexEpoch < 3 and not(h.structuralDrifted))
      h' = h
        .with("indexEpoch", h.indexEpoch + 1)
        .with("structuralDrifted", true)
        .with("derivedActive", if (h.hasAltCleanSupport) h.derivedActive else false)
    else h' = h

  action expandNegativeScopeDomain =
    if (h.rootAdmitted and not(h.negDomainExpanded))
      h' = h.with("negDomainExpanded", true)
    else h' = h

  action reconcileAndRefreshEvidence =
    if (h.oracleVerified and not(h.poisonRetracted) and (h.structuralDrifted or h.negDomainExpanded))
      h' = h
        .with("rootAdmitted", true)
        .with("rootEvidenceEpoch", h.indexEpoch)
        .with("structuralDrifted", false)
        .with("negDomainExpanded", false)
        .with("kvCachedEpoch", h.indexEpoch)
        .with("derivedActive", true)
    else h' = h

  action retractPoisonedEvidence =
    if (h.rootAdmitted and not(h.poisonRetracted))
      h' = h
        .with("poisonRetracted", true)
        .with("rootAdmitted", false)
        .with("derivedActive", if (h.hasAltCleanSupport) h.derivedActive else false)
    else h' = h

  action dropPinnedConstraintInKv =
    if (h.pinnedRetained) h' = h.with("pinnedRetained", false) else h' = h

  action restorePinnedConstraintInKv =
    if (not(h.pinnedRetained)) h' = h.with("pinnedRetained", true) else h' = h

  action serveKvCachedConclusion =
    if (h.kvCachedEpoch > 0 and h.kvCachedEpoch == h.indexEpoch and h.pinnedRetained and not(h.poisonRetracted) and not(h.structuralDrifted) and not(h.negDomainExpanded))
      h' = h.with("validKvServes", h.validKvServes + 1)
    else h' = h

  action consumeDerivedScenarioBChain =
    if (h.rootEvidenceEpoch > 0 and h.derivedActive and (not(h.poisonRetracted) or h.hasAltCleanSupport) and (not(h.structuralDrifted) or h.hasAltCleanSupport))
      h' = h.with("validEvidenceReuses", h.validEvidenceReuses + 1)
    else h' = h

  action step = any {
    verifyWithSearchOracle,
    admitM1Evidence,
    deriveScenarioBChain,
    attachAltCleanJustification,
    applyCommentOnlyEdit,
    reuseStructuralEvidence,
    applyStructuralAstDrift,
    expandNegativeScopeDomain,
    reconcileAndRefreshEvidence,
    retractPoisonedEvidence,
    dropPinnedConstraintInKv,
    restorePinnedConstraintInKv,
    serveKvCachedConclusion,
    consumeDerivedScenarioBChain,
  }

  def safe = all {
    h.staleStructuralReuseViolations == 0,
    h.staleNegDomainReuseViolations == 0,
    h.kvCacheContaminationViolations == 0,
    h.poisonedPromotionViolations == 0,
    h.zombieCascadeViolations == 0,
  }

  // Negative controls (5 adversarial mutations; each must violate `safe`).
  action reuseStaleStructural =
    if (h.rootAdmitted and h.structuralDrifted)
      h' = h.with("staleStructuralReuseViolations", h.staleStructuralReuseViolations + 1)
    else h' = h

  action reuseStaleNegDomain =
    if (h.rootAdmitted and h.negDomainExpanded)
      h' = h.with("staleNegDomainReuseViolations", h.staleNegDomainReuseViolations + 1)
    else h' = h

  action serveContaminatedKv =
    if (h.kvCachedEpoch > 0 and (h.kvCachedEpoch != h.indexEpoch or not(h.pinnedRetained) or h.poisonRetracted or h.structuralDrifted or h.negDomainExpanded))
      h' = h.with("kvCacheContaminationViolations", h.kvCacheContaminationViolations + 1)
    else h' = h

  action admitPoisonedOrUnverified =
    if (not(h.rootAdmitted) and (not(h.oracleVerified) or h.poisonRetracted))
      h' = h.with("rootAdmitted", true).with("poisonedPromotionViolations", h.poisonedPromotionViolations + 1)
    else h' = h

  action consumeZombieDerivedChain =
    if (h.rootEvidenceEpoch > 0 and not(h.derivedActive))
      h' = h.with("zombieCascadeViolations", h.zombieCascadeViolations + 1)
    else h' = h

  action staleStructuralReuseMutation = any { step, reuseStaleStructural }
  action staleNegDomainReuseMutation = any { step, reuseStaleNegDomain }
  action kvCacheContaminationMutation = any { step, serveContaminatedKv }
  action poisonedPromotionMutation = any { step, admitPoisonedOrUnverified }
  action zombieCascadeMutation = any { step, consumeZombieDerivedChain }

  run commentEditPreservesReuseAndRefreshTest =
    init
      .then(verifyWithSearchOracle)
      .then(admitM1Evidence)
      .then(deriveScenarioBChain)
      .then(applyCommentOnlyEdit)
      .then(reuseStructuralEvidence)
      .then(applyStructuralAstDrift)
      .then(reconcileAndRefreshEvidence)
      .then(serveKvCachedConclusion)
      .expect(safe and h.commentOnlyEdit and h.indexEpoch == 2 and h.validEvidenceReuses == 1 and h.validKvServes == 1)

  run atmsAltSupportSurvivesPoisonRetractionTest =
    init
      .then(verifyWithSearchOracle)
      .then(admitM1Evidence)
      .then(deriveScenarioBChain)
      .then(attachAltCleanJustification)
      .then(retractPoisonedEvidence)
      .then(consumeDerivedScenarioBChain)
      .expect(safe and h.poisonRetracted and h.derivedActive and h.validEvidenceReuses == 1)
}
