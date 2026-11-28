// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// Finite protocol model for single-file Org-Babel streaming turns in `tmp/<NNNN>.org`
// and zero-compaction evolution into GitOps + `Artifacts` + `marlin-agent-storage`.
// Companion Org Contract templates: ../../scheme/context-system/org-contract/templates/README.org
module orgBabelFission {
  type TurnState =
    | Composing
    | PromptCommitted
    | Streaming
    | Completed
    | RefiledStub

  type TurnRecord = {
    id: int,
    state: TurnState,
    lines: int,
    digest: int,
    topicReady: bool
  }

  type PromotedArtifact = {
    originTurnId: int,
    collection: str,
    digest: int,
    contractAdmitted: bool,
    sodAdmitted: bool
  }

  pure val MAX_SCRATCH_LINES = 1000
  pure val STUB_LINES = 6
  pure val TURN_IDS = Set(1, 2, 3, 4)

  var nextTurnId: int
  var scratchTurns: int -> TurnRecord
  var activeTurnIds: Set[int]
  var committedDigests: Set[int]
  var orgStoreArtifacts: Set[PromotedArtifact]
  var compactorRan: bool

  pure def scratchLineCount(ids: Set[int], table: int -> TurnRecord): int =
    ids.fold(0, (acc, id) => acc + table.get(id).lines)

  pure def retainedDigests(
    ids: Set[int],
    table: int -> TurnRecord,
    promoted: Set[PromotedArtifact]
  ): Set[int] = {
    val inlineDigests =
      ids
        .filter(id =>
          table.get(id).state == PromptCommitted
            or table.get(id).state == Streaming
            or table.get(id).state == Completed
        )
        .map(id => table.get(id).digest)
    val storeDigests = promoted.map(a => a.digest)
    inlineDigests.union(storeDigests)
  }

  action init = all {
    nextTurnId' = 1,
    scratchTurns' = Map(),
    activeTurnIds' = Set(),
    committedDigests' = Set(),
    orgStoreArtifacts' = Set(),
    compactorRan' = false,
  }

  // User submits a `#+BEGIN_SRC llm :model ... :thinking ...` block in `tmp/<NNNN>.org`.
  // Admission pauses new turns if quiescent completed turns already pushed scratch over watermark
  // until zero-compaction structural fission extracts them.
  action submitBabelPrompt(promptLines: int): bool = all {
    nextTurnId <= 4,
    scratchLineCount(activeTurnIds, scratchTurns) < MAX_SCRATCH_LINES,
    val id = nextTurnId
    val rec: TurnRecord = {
      id: id,
      state: PromptCommitted,
      lines: promptLines,
      digest: id * 100,
      topicReady: false,
    }
    all {
      nextTurnId' = nextTurnId + 1,
      scratchTurns' = scratchTurns.put(id, rec),
      activeTurnIds' = activeTurnIds.union(Set(id)),
      committedDigests' = committedDigests.union(Set(rec.digest)),
      orgStoreArtifacts' = orgStoreArtifacts,
      compactorRan' = compactorRan,
    }
  }

  // Provider begins streaming into affiliated `#+RESULTS: turn-<id>`.
  action streamResultsDelta(id: int, deltaLines: int): bool = all {
    activeTurnIds.contains(id),
    val cur = scratchTurns.get(id)
    all {
      cur.state == PromptCommitted or cur.state == Streaming,
      val updated: TurnRecord = {
        ...cur,
        state: Streaming,
        lines: cur.lines + deltaLines,
      }
      all {
        scratchTurns' = scratchTurns.put(id, updated),
        nextTurnId' = nextTurnId,
        activeTurnIds' = activeTurnIds,
        committedDigests' = committedDigests,
        orgStoreArtifacts' = orgStoreArtifacts,
        compactorRan' = compactorRan,
      }
    }
  }

  // Streaming completes; `#+RESULTS:` is sealed and topic may crystallize.
  action finishBabelTurn(id: int, topicCrystallized: bool): bool = all {
    activeTurnIds.contains(id),
    val cur = scratchTurns.get(id)
    all {
      cur.state == Streaming,
      val updated: TurnRecord = {
        ...cur,
        state: Completed,
        topicReady: topicCrystallized,
      }
      all {
        scratchTurns' = scratchTurns.put(id, updated),
        nextTurnId' = nextTurnId,
        activeTurnIds' = activeTurnIds,
        committedDigests' = committedDigests,
        orgStoreArtifacts' = orgStoreArtifacts,
        compactorRan' = compactorRan,
      }
    }
  }

  // Zero-compaction structural fission / governed refile:
  // Triggered either when topicReady holds OR when scratchLineCount >= MAX_SCRATCH_LINES.
  // Extracts a quiescent (`Completed`) subtree to `OrgStore` after Contract + SoD gate,
  // and replaces the inline section in `tmp/` with a 6-line digest-pinned stub.
  action structuralFissionRefile(id: int, targetCollection: str): bool = all {
    activeTurnIds.contains(id),
    val cur = scratchTurns.get(id)
    val overWatermark = scratchLineCount(activeTurnIds, scratchTurns) >= MAX_SCRATCH_LINES
    all {
      cur.state == Completed,
      cur.topicReady or overWatermark,
      val artifact: PromotedArtifact = {
        originTurnId: id,
        collection: targetCollection,
        digest: cur.digest,
        contractAdmitted: true,
        sodAdmitted: true,
      }
      val stub: TurnRecord = {
        ...cur,
        state: RefiledStub,
        lines: STUB_LINES,
      }
      all {
        scratchTurns' = scratchTurns.put(id, stub),
        orgStoreArtifacts' = orgStoreArtifacts.union(Set(artifact)),
        nextTurnId' = nextTurnId,
        activeTurnIds' = activeTurnIds,
        committedDigests' = committedDigests,
        compactorRan' = compactorRan,
      }
    }
  }

  action step = {
    nondet id = TURN_IDS.oneOf()
    nondet col = Set("work", "evidence", "topology").oneOf()
    nondet ready = Set(true, false).oneOf()
    any {
      submitBabelPrompt(120),
      streamResultsDelta(id, 460),
      finishBabelTurn(id, ready),
      structuralFissionRefile(id, col),
    }
  }

  // --- Negative-control mutations (each MUST violate `safe`) ---

  // Mutant 1: Lossy `/compact` overwrites a completed turn with a summary, losing its original digest!
  action lossyCompactMutation(id: int): bool = all {
    activeTurnIds.contains(id),
    scratchTurns.get(id).state == Completed,
    val cur = scratchTurns.get(id)
    val summarized: TurnRecord = {
      ...cur,
      lines: 20,
      digest: -1,
    }
    all {
      scratchTurns' = scratchTurns.put(id, summarized),
      compactorRan' = true,
      nextTurnId' = nextTurnId,
      activeTurnIds' = activeTurnIds,
      committedDigests' = committedDigests,
      orgStoreArtifacts' = orgStoreArtifacts,
    }
  }

  // Mutant 2: Splitting/refiling an in-flight `Streaming` Babel block mid-stream!
  action midStreamRefileMutation(id: int): bool = all {
    activeTurnIds.contains(id),
    scratchTurns.get(id).state == Streaming,
    val cur = scratchTurns.get(id)
    val prematureStub: TurnRecord = {
      ...cur,
      state: RefiledStub,
      lines: STUB_LINES,
    }
    all {
      scratchTurns' = scratchTurns.put(id, prematureStub),
      nextTurnId' = nextTurnId,
      activeTurnIds' = activeTurnIds,
      committedDigests' = committedDigests,
      orgStoreArtifacts' = orgStoreArtifacts,
      compactorRan' = compactorRan,
    }
  }

  // Mutant 3: Promoting scratch content into governed `OrgStore` without Contract / SoD gate!
  action ungatedPromoteMutation(id: int): bool = all {
    activeTurnIds.contains(id),
    scratchTurns.get(id).state == Completed,
    val cur = scratchTurns.get(id)
    val badArtifact: PromotedArtifact = {
      originTurnId: id,
      collection: "work",
      digest: cur.digest,
      contractAdmitted: false,
      sodAdmitted: false,
    }
    val stub: TurnRecord = { ...cur, state: RefiledStub, lines: STUB_LINES }
    all {
      scratchTurns' = scratchTurns.put(id, stub),
      orgStoreArtifacts' = orgStoreArtifacts.union(Set(badArtifact)),
      nextTurnId' = nextTurnId,
      activeTurnIds' = activeTurnIds,
      committedDigests' = committedDigests,
      compactorRan' = compactorRan,
    }
  }

  action lossyCompactStep = {
    nondet id = TURN_IDS.oneOf()
    any { step, lossyCompactMutation(id) }
  }

  action midStreamRefileStep = {
    nondet id = TURN_IDS.oneOf()
    any { step, midStreamRefileMutation(id) }
  }

  action ungatedPromoteStep = {
    nondet id = TURN_IDS.oneOf()
    any { step, ungatedPromoteMutation(id) }
  }

  // --- Core Safety Invariants ---

  // 1. MAR-BBL-R03: Zero lossy compaction; every committed turn digest is 100% conserved
  //    either inline in `tmp/` or in an admitted `OrgStore` target collection.
  def evidenceConserved: bool =
    not(compactorRan)
      and committedDigests == retainedDigests(activeTurnIds, scratchTurns, orgStoreArtifacts)

  // 2. MAR-BBL-R03 / MAR-BBL-R05: Every `RefiledStub` in `tmp/` points to an existing
  //    promoted `OrgStore` artifact with identical digest (never orphaned or mid-stream).
  def stubsConsistent: bool =
    activeTurnIds.forall(id =>
      val t = scratchTurns.get(id)
      (t.state == RefiledStub) implies
        orgStoreArtifacts.exists(a => a.originTurnId == id and a.digest == t.digest)
    )

  // 3. MAR-BBL-R04: Every promoted artifact in `OrgStore` passed Contract and SoD gates.
  def promotionGoverned: bool =
    orgStoreArtifacts.forall(a => a.contractAdmitted and a.sodAdmitted)

  def safe: bool = all {
    evidenceConserved,
    stubsConsistent,
    promotionGoverned,
  }

  // Deterministic witness: two big Babel turns push `tmp/0042.org` over 1,000 lines (580 + 580 = 1,160),
  // triggering structural fission into `work` -> active `tmp/` drops to 586 lines (< 1,000) with
  // 100% digest conservation, allowing Turn 3 to proceed cleanly!
  run watermarkFissionWitnessTest =
    init
      .then(submitBabelPrompt(120))
      .then(streamResultsDelta(1, 460))
      .then(finishBabelTurn(1, false))
      .then(submitBabelPrompt(120))
      .then(streamResultsDelta(2, 460))
      .then(finishBabelTurn(2, true))
      .then(all {
        assert(scratchLineCount(activeTurnIds, scratchTurns) == 1160),
        structuralFissionRefile(2, "work"),
      })
      .then(all {
        assert(scratchLineCount(activeTurnIds, scratchTurns) == 586),
        assert(safe),
        submitBabelPrompt(120),
      })
}
