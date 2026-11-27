// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/engineering/defense/tool-sandbox-rpc-boundary/ToolSandboxRpcBoundary"
// @countermodel marlin-offer-pins-schema-digest step=stepMarlinSchemaRugPull max-steps=6 itf="h.rugPullViolations > 0"
// @countermodel marlin-offer-rejects-unpinned-shadow-tool step=stepMarlinShadowToolBypass max-steps=6 itf="h.shadowToolViolations > 0"

// Marlin B11 Tool-Sandbox & Offer Schema-Pinning overlay composing
// ToolSandboxRpcBoundary with marlin-agent-runtime's ModelToolOffer SHA-256
// schema-digest pinning (verify_pinned_schema / SchemaRugPullDetected).
module marlin_backend_tool_sandbox_overlay {
  type State = {
    offerFrozen: bool,
    pinnedDigestMatched: bool,
    shadowToolRequested: bool,
    shadowToolViolations: int
  }

  action init = h' = {
    offerFrozen: false,
    pinnedDigestMatched: false,
    shadowToolRequested: false,
    shadowToolViolations: 0
  }

  // ModelToolCatalog::offer freezes the SHA-256 schema digest for every registered tool.
  action freezeMarlinToolOffer =
    if (not(h.offerFrozen))
      h' = h
        .with("offerFrozen", true)
        .with("manifestPinned", true)
        .with("pinnedDigestMatched", true)
    else
      h' = h

  // Observed tool definition mutates description or JSON schema after offer freezing (MCP rug-pull).
  action driftObservedToolSchema =
    if (h.offerFrozen and not(h.schemaRugPulled))
      h' = h
        .with("schemaRugPulled", true)
        .with("pinnedDigestMatched", false)
    else
      h' = h

  // Provider requests an unpinned shadow tool not present in ModelToolOffer::index.
  action requestShadowTool =
    if (h.offerFrozen and not(h.shadowToolRequested))
      h' = h.with("shadowToolRequested", true)
    else
      h' = h

  // Safe execution requires frozen offer, intact SHA-256 schema digest, no shadow tool,
  // valid scope, and no pending ambiguous RPC timeout.
  action executePinnedMarlinToolSafe =
    if (h.safeExecutions < 3 and h.offerFrozen and h.manifestPinned and not(h.schemaRugPulled) and h.pinnedDigestMatched and not(h.shadowToolRequested) and h.scopeValid and not(h.rpcTimeoutPending))
      h' = h.with("safeExecutions", h.safeExecutions + 1)
    else
      h' = h

  action step = any {
    freezeMarlinToolOffer,
    grantValidScope,
    driftObservedToolSchema,
    requestShadowTool,
    triggerAmbiguousTimeout,
    reconcileTimeout,
    executePinnedMarlinToolSafe
  }

  val safety = (
    h.rugPullViolations == 0 and
    h.scopeEscapeViolations == 0 and
    h.blindRetryViolations == 0 and
    h.shadowToolViolations == 0
  )

  // Countermodel: bypassing verify_pinned_schema allows a rug-pulled tool definition to execute.
  action executeUnverifiedRugPull =
    if (h.safeExecutions < 3 and h.offerFrozen and h.schemaRugPulled and h.scopeValid and not(h.rpcTimeoutPending))
      h' = h
        .with("safeExecutions", h.safeExecutions + 1)
        .with("rugPullViolations", h.rugPullViolations + 1)
    else
      h' = h

  action stepMarlinSchemaRugPull = any { step, executeUnverifiedRugPull }

  // Countermodel: bypassing ModelToolOffer::index admits an unpinned shadow tool.
  action executeShadowToolBypass =
    if (h.safeExecutions < 3 and h.offerFrozen and h.shadowToolRequested and h.scopeValid and not(h.rpcTimeoutPending))
      h' = h
        .with("safeExecutions", h.safeExecutions + 1)
        .with("shadowToolViolations", h.shadowToolViolations + 1)
    else
      h' = h

  action stepMarlinShadowToolBypass = any { step, executeShadowToolBypass }
}
