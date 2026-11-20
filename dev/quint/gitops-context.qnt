// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// MAR-GCTX-R01/R02/R03/R06/R07. Both modeled clones have the same URL.
// An owner generation stands for changes to selected bytes or their binding.
module gitopsContextAdmission {
  type State = {
    repo: int, generation: int, indexedGeneration: int, cursor: int,
    grant: bool, grantEpoch: int, requestedRepo: int, requestedCut: int,
    requestedGrant: int, read: bool, visible: bool, forbiddenRead: bool
  }
  var s: State

  action init = s' = {
    repo: 1, generation: 1, indexedGeneration: 1, cursor: 1,
    grant: true, grantEpoch: 1, requestedRepo: 0, requestedCut: 0,
    requestedGrant: 0, read: false, visible: false, forbiddenRead: false
  }

  def requestCurrent = all {
    s.repo == 1, s.requestedRepo == s.repo, s.requestedCut == s.generation,
    s.requestedGrant == s.grantEpoch, s.grant,
    s.indexedGeneration == s.generation, s.cursor == s.generation
  }
  action requestRestore = s' = s
    .with("requestedRepo", s.repo).with("requestedCut", s.indexedGeneration)
    .with("requestedGrant", s.grantEpoch).with("read", false).with("visible", false)
  action switchClone = all {
    s.repo == 1, s' = s.with("repo", 2).with("visible", false)
  }
  action editBytes = all {
    s.generation < 3,
    s' = s.with("generation", s.generation + 1).with("visible", false)
  }
  action replayOwnerDelta = s' = s
    .with("indexedGeneration", s.generation).with("cursor", s.generation)
  action revoke = all {
    s.grant, s.grantEpoch == 1, s' = s.with("grant", false)
      .with("grantEpoch", s.grantEpoch + 1).with("visible", false)
  }
  action grantAgain = all {
    not(s.grant), s' = s.with("grant", true).with("grantEpoch", 3)
  }
  action readSource = all {
    requestCurrent, s' = s.with("read", true)
  }
  action publish = all {
    s.read, requestCurrent, s' = s.with("visible", true)
  }
  action step = any {
    requestRestore, switchClone, editBytes, replayOwnerDelta, revoke, grantAgain,
    readSource, publish
  }
  def safe = all {
    s.indexedGeneration == s.cursor, s.cursor <= s.generation,
    not(s.forbiddenRead), s.visible implies (s.read and requestCurrent)
  }

  action readForeignClone = all {
    s.repo == 2, s.requestedRepo == 2, s.grant,
    s' = s.with("read", true).with("forbiddenRead", true)
  }
  action publishStale = all {
    s.read, not(requestCurrent), s' = s.with("visible", true)
  }
  action commitCursorAlone = all {
    s.generation > s.cursor, s' = s.with("cursor", s.generation)
  }
  action foreignCloneMutation = any { step, readForeignClone }
  action stalePublicationMutation = any { step, publishStale }
  action cursorMutation = any { step, commitCursorAlone }

  run authorizedRestoreTest = init.then(requestRestore).then(readSource)
    .then(publish).expect(safe and s.visible)
  run revokedGrantWithdrawsViewTest = init.then(requestRestore).then(readSource)
    .then(publish).then(revoke).expect(safe and not(s.visible))
    .then(grantAgain).expect(safe and not(requestCurrent))
  run newBytesRequireReplayAndNewRequestTest = init.then(requestRestore)
    .then(readSource).then(editBytes).expect(safe and not(requestCurrent))
    .then(replayOwnerDelta).expect(safe and not(requestCurrent))
    .then(requestRestore).then(readSource).then(publish).expect(safe and s.visible)
  run sameUrlCloneCannotReuseOwnerTest = init.then(switchClone)
    .then(requestRestore).expect(safe and not(requestCurrent))
}

// MAR-GCTX-R04. One owned Git retention ref or one captured object.
// tip 1 is named by the admitted merge receipt, tip 2 is a later branch cut.
module gitopsContextRetention {
  type State = {
    tip: int, active: bool, retiredTip: int, merged: bool, bytes: bool,
    hold: bool, linked: bool, expired: bool, complete: bool,
    ownerGeneration: int, ticket: int, disposed: bool
  }
  var s: State
  action init = s' = {
    tip: 1, active: true, retiredTip: 0, merged: false, bytes: true,
    hold: false, linked: false, expired: false, complete: true,
    ownerGeneration: 1, ticket: 0, disposed: false
  }

  action advanceTip = all { s.active, s.tip == 1, s' = s.with("tip", 2) }
  action mergeTipOne = s' = if (s.active and s.tip == 1)
    s.with("active", false).with("retiredTip", 1).with("merged", true) else s
  action deleteRef = all {
    s.active, s' = s.with("active", false).with("retiredTip", s.tip)
  }
  action acquireHold = all {
    s.bytes, not(s.hold), s.ownerGeneration < 4,
    s' = s.with("hold", true).with("ownerGeneration", s.ownerGeneration + 1)
  }
  action publishLink = all { s.hold, s.bytes, s' = s.with("linked", true) }
  action withdrawLink = all { s.linked, s' = s.with("linked", false) }
  action releaseHold = all {
    s.hold, not(s.linked), s.ownerGeneration < 4,
    s' = s.with("hold", false).with("ownerGeneration", s.ownerGeneration + 1)
  }
  action authorizeExpiry = all { not(s.active), s' = s.with("expired", true) }
  action loseCoverage = s' = if (s.bytes) s.with("complete", false) else s
  def eligible = not(s.active) and s.expired and s.complete and not(s.hold) and s.bytes
  action prepareDisposal = all {
    eligible, s' = s.with("ticket", s.ownerGeneration)
  }
  action dispose = all {
    eligible, s.ticket == s.ownerGeneration,
    s' = s.with("bytes", false).with("disposed", true)
  }
  action step = any {
    advanceTip, mergeTipOne, deleteRef, acquireHold, publishLink, withdrawLink,
    releaseHold, authorizeExpiry, loseCoverage, prepareDisposal, dispose
  }
  def safe = all {
    s.linked implies s.hold, s.hold implies s.bytes,
    s.merged implies (s.tip == 1 and s.retiredTip == 1 and not(s.active)),
    s.disposed implies (
      not(s.active) and s.expired and s.complete and not(s.hold)
      and not(s.bytes) and s.ticket == s.ownerGeneration
    )
  }
  action retireAdvancedBranch = all {
    s.active, s.tip == 2,
    s' = s.with("active", false).with("retiredTip", s.tip).with("merged", true)
  }
  action useOldDisposalTicket = all {
    s.bytes, s.ticket > 0, s.ticket < s.ownerGeneration,
    s' = s.with("bytes", false).with("disposed", true)
  }
  action mergeMutation = any { step, retireAdvancedBranch }
  action custodyMutation = any { step, useOldDisposalTicket }

  run mergeDoesNotRetireLaterTipTest = init.then(advanceTip)
    .then(mergeTipOne).expect(safe and s.active)
  run deletionPreservesMergeDistinctionTest = init.then(deleteRef)
    .expect(safe and not(s.active) and not(s.merged))
  run referenceFencesDisposalTest = init.then(mergeTipOne).then(authorizeExpiry)
    .then(prepareDisposal).then(acquireHold).then(publishLink)
    .expect(safe and not(eligible) and s.ticket < s.ownerGeneration)
  run currentOwnerCanDisposeTest = init.then(mergeTipOne).then(acquireHold)
    .then(publishLink).then(withdrawLink).then(releaseHold).then(authorizeExpiry)
    .then(prepareDisposal).then(dispose).expect(safe and s.disposed)
  run coverageGapRetainsBytesTest = init.then(mergeTipOne).then(authorizeExpiry)
    .then(prepareDisposal).then(loseCoverage).expect(safe and not(eligible) and s.bytes)
}

// MAR-GCTX-R07. Lab's link resolver and the Git-bound acceptance claim
// have distinct inputs. A shared Worktree label supplies no Git evidence.
module gitopsOrgLinkEvidence {
  type LinkCase = {
    orgLink: bool, relativeFile: bool, declared: bool, uniqueSelector: bool,
    currentCut: bool, targetGrant: bool, selectedWorktree: bool, repoBound: bool
  }
  val localLink: LinkCase = {
    orgLink: true, relativeFile: true, declared: true, uniqueSelector: true,
    currentCut: true, targetGrant: true, selectedWorktree: true, repoBound: true
  }
  val attachment = localLink.with("relativeFile", false)
  val undeclared = localLink.with("declared", false)
  val otherRepo = localLink.with("repoBound", false)
  val stale = localLink.with("currentCut", false)
  val denied = localLink.with("targetGrant", false)
  val ambiguous = localLink.with("uniqueSelector", false)
  val foreignWorktree = localLink.with("selectedWorktree", false)
  val sourceBlock = localLink.with("orgLink", false)

  var selected: LinkCase
  var resolved: bool
  var gitQualified: bool
  action init = all {
    selected' = sourceBlock, resolved' = false, gitQualified' = false
  }
  action inspect(link: LinkCase): bool = all {
    selected' = link, resolved' = false, gitQualified' = false
  }
  def eligible = all {
    selected.orgLink, selected.relativeFile, selected.declared, selected.uniqueSelector,
    selected.currentCut, selected.targetGrant, selected.selectedWorktree
  }
  action resolve = all {
    eligible, selected' = selected, resolved' = true, gitQualified' = gitQualified
  }
  action qualifyGitBinding = all {
    resolved, selected.repoBound, selected' = selected, resolved' = resolved, gitQualified' = true
  }
  action step = any {
    inspect(localLink), inspect(attachment), inspect(undeclared), inspect(otherRepo),
    inspect(stale), inspect(denied), inspect(ambiguous), inspect(foreignWorktree),
    inspect(sourceBlock), resolve, qualifyGitBinding
  }
  def safe = all {
    resolved implies eligible, gitQualified implies (resolved and selected.repoBound)
  }
  action followAttachment = all {
    selected == attachment, selected' = selected, resolved' = true, gitQualified' = false
  }
  action creditUnboundRepo = all {
    resolved, not(selected.repoBound), selected' = selected, resolved' = resolved, gitQualified' = true
  }
  action attachmentMutation = any { step, followAttachment }
  action crossRepoMutation = any { step, creditUnboundRepo }

  run declaredFileCanQualifyTest = init.then(inspect(localLink)).then(resolve)
    .then(qualifyGitBinding).expect(safe and gitQualified)
  run renderedLinksHaveNoAutomaticResolutionTest = init.then(inspect(attachment))
    .expect(selected.orgLink and not(eligible)).then(inspect(undeclared))
    .expect(selected.orgLink and not(eligible) and not(resolved))
  run labResolutionAloneCannotQualifyRepoTest = init.then(inspect(otherRepo)).then(resolve)
    .expect(safe and resolved and not(selected.repoBound) and not(gitQualified))
  run localLinkAdmissionInputsTest = init.then(inspect(stale)).expect(not(eligible))
    .then(inspect(denied)).expect(not(eligible)).then(inspect(ambiguous)).expect(not(eligible))
    .then(inspect(foreignWorktree)).expect(not(eligible))
    .then(inspect(sourceBlock)).expect(not(eligible) and not(selected.orgLink))
}

// MAR-GCTX-R07. PR 34 cross-Repo publication uses an explicit, pinned origin.
// Local file resolution above cannot supply this evidence.
module gitopsCrossRepoPublication {
  type Evidence = {
    sourceBound: bool, targetBound: bool, declaredRelation: bool,
    contractsMatch: bool, sourceCurrent: bool, targetCurrent: bool,
    sourceGrant: bool, targetGrant: bool
  }
  val qualified: Evidence = {
    sourceBound: true, targetBound: true, declaredRelation: true,
    contractsMatch: true, sourceCurrent: true, targetCurrent: true,
    sourceGrant: true, targetGrant: true
  }
  var evidence: Evidence
  var prepared: bool
  // Open evidence is current only until a binding or grant changes.
  var opened: int
  var inFlight: bool
  var admittedRead: bool
  // Generation one represents the already authorized warm cache.
  var cacheGeneration: int
  var pending: bool
  var published: bool

  def readable = all {
    evidence.sourceBound, evidence.targetBound, evidence.sourceCurrent, evidence.targetCurrent,
    evidence.sourceGrant, evidence.targetGrant
  }
  def eligible = readable and evidence.declaredRelation and evidence.contractsMatch
  action selectOrigin(nextEvidence: Evidence): bool = all {
    evidence' = nextEvidence, prepared' = false, opened' = 0,
    inFlight' = false, admittedRead' = false, cacheGeneration' = 1,
    pending' = false, published' = false
  }
  action init = selectOrigin(qualified.with("declaredRelation", false))
  action prepare = all {
    readable, not(inFlight), not(admittedRead),
    evidence' = evidence, prepared' = true, opened' = 0,
    inFlight' = false, admittedRead' = admittedRead, cacheGeneration' = cacheGeneration,
    pending' = pending, published' = published
  }
  action attemptOpen = all {
    not(inFlight), not(admittedRead),
    evidence' = evidence, prepared' = prepared,
    opened' = if (readable and prepared) 2 else 0,
    inFlight' = (readable and prepared),
    admittedRead' = admittedRead, cacheGeneration' = cacheGeneration,
    pending' = pending, published' = published
  }
  action deliverRead = all {
    inFlight, evidence' = evidence, prepared' = prepared, opened' = opened,
    inFlight' = false, admittedRead' = eligible,
    cacheGeneration' = if (eligible) 2 else cacheGeneration,
    pending' = pending, published' = published
  }
  action start = all {
    eligible, admittedRead, evidence' = evidence, prepared' = prepared,
    inFlight' = inFlight, admittedRead' = admittedRead, cacheGeneration' = cacheGeneration,
    opened' = opened, pending' = true, published' = published
  }
  action publish = all {
    pending, eligible, evidence' = evidence, prepared' = prepared,
    inFlight' = inFlight, admittedRead' = admittedRead, cacheGeneration' = cacheGeneration,
    opened' = opened, pending' = false, published' = true
  }
  // Prior admission can outlive the authority it observed. Keep its token
  // and the pending evaluation to exercise both recheck points.
  action invalidate(nextEvidence: Evidence): bool = all {
    evidence' = nextEvidence, prepared' = prepared, opened' = 0,
    inFlight' = inFlight, admittedRead' = admittedRead, cacheGeneration' = cacheGeneration,
    pending' = pending, published' = false
  }
  action advanceSource = invalidate(evidence.with("sourceCurrent", false))
  action advanceTarget = invalidate(evidence.with("targetCurrent", false))
  action revokeSource = invalidate(evidence.with("sourceGrant", false))
  action revokeTarget = invalidate(evidence.with("targetGrant", false))
  action step = any {
    selectOrigin(qualified), selectOrigin(qualified.with("sourceBound", false)),
    selectOrigin(qualified.with("targetBound", false)), selectOrigin(qualified.with("declaredRelation", false)),
    selectOrigin(qualified.with("contractsMatch", false)),
    selectOrigin(qualified.with("sourceGrant", false)),
    selectOrigin(qualified.with("targetGrant", false)),
    prepare, attemptOpen, deliverRead, start, publish,
    advanceSource, advanceTarget, revokeSource, revokeTarget
  }
  def safe = all {
    published implies (eligible and admittedRead),
    opened != 0 implies (opened == 2 and readable and prepared),
    (cacheGeneration == 2) iff admittedRead,
    pending implies admittedRead
  }
  action publishAfterInvalidation = all {
    pending, not(eligible), evidence' = evidence, prepared' = prepared,
    inFlight' = inFlight, admittedRead' = admittedRead, cacheGeneration' = cacheGeneration,
    opened' = opened, pending' = false, published' = true
  }
  action openFromStalePreparation = all {
    prepared, not(readable), evidence' = evidence, prepared' = prepared,
    inFlight' = inFlight, admittedRead' = admittedRead, cacheGeneration' = cacheGeneration,
    opened' = 2, pending' = pending, published' = published
  }
  action cacheAfterRevocation = all {
    inFlight, not(readable), evidence' = evidence, prepared' = prepared, opened' = opened,
    inFlight' = false, admittedRead' = false, cacheGeneration' = 2,
    pending' = pending, published' = published
  }
  action latePublicationMutation = any {
    selectOrigin(qualified), prepare, attemptOpen, deliverRead, start, revokeTarget,
    publishAfterInvalidation
  }
  action preOpenMutation = any { step, openFromStalePreparation }
  action lateCacheMutation = any {
    selectOrigin(qualified), prepare, attemptOpen, revokeSource, cacheAfterRevocation
  }

  run bothGitSourcesAndGrantsTest = init.then(selectOrigin(qualified)).then(prepare).then(attemptOpen)
    .expect(opened == 2 and inFlight and cacheGeneration == 1)
    .then(deliverRead).expect(cacheGeneration == 2 and admittedRead).then(start).then(publish)
    .expect(published and eligible).then(advanceTarget).expect(not(published) and not(eligible))
    .then(selectOrigin(qualified)).then(prepare).then(attemptOpen).then(deliverRead)
    .then(start).then(publish).then(revokeSource).expect(not(published))
  run originAndContractControlsTest = init
    .then(selectOrigin(qualified.with("sourceBound", false))).expect(not(eligible))
    .then(selectOrigin(qualified.with("targetBound", false))).expect(not(eligible))
    .then(selectOrigin(qualified.with("contractsMatch", false))).expect(not(eligible))
    .then(selectOrigin(qualified.with("declaredRelation", false))).expect(not(eligible))
  run pendingTargetGrantRevocationTest = init.then(selectOrigin(qualified)).then(prepare).then(attemptOpen)
    .then(deliverRead).then(start).then(revokeTarget)
    .expect(pending and not(eligible) and not(published))
  run missingGrantsBlockBothOpensTest = init
    .then(selectOrigin(qualified.with("sourceGrant", false))).then(attemptOpen).expect(opened == 0)
    .then(selectOrigin(qualified.with("targetGrant", false))).then(attemptOpen).expect(opened == 0)
  run revokedPreparationBlocksBothOpensTest = init
    .then(selectOrigin(qualified)).then(prepare).then(revokeSource).then(attemptOpen)
    .expect(prepared and opened == 0)
    .then(selectOrigin(qualified)).then(prepare).then(revokeTarget).then(attemptOpen)
    .expect(prepared and opened == 0)
  run revokedInflightDeliveryLeavesWarmCacheTest = init
    .then(selectOrigin(qualified)).then(prepare).then(attemptOpen).then(revokeSource)
    .expect(inFlight and not(readable)).then(deliverRead)
    .expect(not(admittedRead) and not(pending) and not(published) and cacheGeneration == 1)
    .then(selectOrigin(qualified)).then(prepare).then(attemptOpen).then(revokeTarget)
    .expect(inFlight and not(readable)).then(deliverRead)
    .expect(not(admittedRead) and not(pending) and not(published) and cacheGeneration == 1)
    .then(selectOrigin(qualified)).then(prepare).then(attemptOpen).then(deliverRead)
    .expect(admittedRead and cacheGeneration == 2)
}

// MAR-GCTX-R09. Feature admission consumes an existing GitOps core policy.
module gitopsWorkingCopyPolicy {
  var hasCorePolicy: bool
  var wantsWorktrunk: bool
  // 0 is refused, 1 is the core provider, 2 is the WorkTrunk refinement.
  var selected: int

  action init = all { hasCorePolicy' = false, wantsWorktrunk' = false, selected' = 0 }
  action declareCore(present: bool, worktrunk: bool): bool = all {
    hasCorePolicy' = present, wantsWorktrunk' = worktrunk, selected' = 0
  }
  action admit = all {
    hasCorePolicy' = hasCorePolicy, wantsWorktrunk' = wantsWorktrunk,
    selected' = if (hasCorePolicy) (if (wantsWorktrunk) 2 else 1) else 0
  }
  action step = any {
    declareCore(false, false), declareCore(false, true),
    declareCore(true, false), declareCore(true, true), admit
  }
  def safe = all {
    selected > 0 implies hasCorePolicy, selected == 2 implies wantsWorktrunk
  }
  action admitBareFeature = all {
    not(hasCorePolicy), wantsWorktrunk, hasCorePolicy' = hasCorePolicy,
    wantsWorktrunk' = wantsWorktrunk, selected' = 2
  }
  action missingCoreMutation = any { step, admitBareFeature }

  run basePolicyTest = init.then(declareCore(true, false)).then(admit)
    .expect(safe and selected == 1)
  run worktrunkRefinementTest = init.then(declareCore(true, true)).then(admit)
    .expect(safe and selected == 2)
  run fixtureMissingPolicyIsRefusedTest = init.then(declareCore(false, true)).then(admit)
    .expect(safe and selected == 0)
}
