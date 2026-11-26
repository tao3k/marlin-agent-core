// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/threat/zero-trust/shared-context-governance/SharedContextGovernance"
// @countermodel cross-tenant-context-isolation step=stepCrossTenantReadLeak max-steps=6 itf="h.crossTenantLeak == true"
// @countermodel dm-allowlist-isolation step=stepDmOutsiderReadLeak max-steps=6 itf="h.dmOutsiderLeak == true"
// @countermodel prompt-scope-anti-exfiltration step=stepPromptScopeExfiltration max-steps=6 itf="h.exfiltrationLeak == true"

// Marlin Rust backend marlin-agent-context 5-scope overlay composing upstream
// SharedContextGovernance with multi-tenant isolation, DM participant allowlist,
// and prompt-bundle anti-exfiltration guarantees.
module marlin_backend_shared_context_overlay {
  type State = {
    dmShared: bool,
    crossTenantLeak: bool,
    dmOutsiderLeak: bool,
    exfiltrationLeak: bool
  }

  action init = h' = {
    dmShared: false,
    crossTenantLeak: false,
    dmOutsiderLeak: false,
    exfiltrationLeak: false
  }

  action shareToDirectMessageAs(actorIsOwner: bool, explicitDeclassify: bool): bool =
    if (h.entryScope == 0 and not(h.dmShared) and actorIsOwner and (not(h.entryConfidential) or explicitDeclassify))
      h' = h.with("dmShared", true).with("entryDeclassified", h.entryDeclassified or explicitDeclassify)
    else
      h' = h

  action shareToDirectMessage = {
    nondet actorIsOwner = Set(false, true).oneOf()
    nondet explicitDeclassify = Set(false, true).oneOf()
    shareToDirectMessageAs(actorIsOwner, explicitDeclassify)
  }

  action readDirectMessageAs(sameTenant: bool, isDmParticipant: bool): bool =
    if (h.dmShared and sameTenant and isDmParticipant and h.peerReads < 3)
      h' = h
        .with("peerReads", h.peerReads + 1)
        .with("crossTenantLeak", h.crossTenantLeak or not(sameTenant))
        .with("dmOutsiderLeak", h.dmOutsiderLeak or not(isDmParticipant))
    else
      h' = h

  action readDirectMessage = {
    nondet sameTenant = Set(false, true).oneOf()
    nondet isDmParticipant = Set(false, true).oneOf()
    readDirectMessageAs(sameTenant, isDmParticipant)
  }

  action assembleChannelPromptSafe =
    if (h.entryScope >= 1)
      h' = h.with("exfiltrationLeak", h.exfiltrationLeak or h.entryScope == 0)
    else
      h' = h

  action step = any {
    shareToDirectMessage,
    readDirectMessage,
    assembleChannelPromptSafe
  }

  val safety =
    not(h.crossTenantLeak)
    and not(h.dmOutsiderLeak)
    and not(h.exfiltrationLeak)

  action readCrossTenantUnchecked =
    if ((h.dmShared or h.entryScope >= 1) and h.peerReads < 3)
      h' = h.with("peerReads", h.peerReads + 1).with("crossTenantLeak", true)
    else
      h' = h

  action stepCrossTenantReadLeak = any { step, readCrossTenantUnchecked }

  action readDmOutsiderUnchecked =
    if (h.dmShared and h.peerReads < 3)
      h' = h.with("peerReads", h.peerReads + 1).with("dmOutsiderLeak", true)
    else
      h' = h

  action stepDmOutsiderReadLeak = any { step, readDmOutsiderUnchecked }

  action assembleChannelPromptUnchecked =
    if (h.entryScope == 0)
      h' = h.with("exfiltrationLeak", true)
    else
      h' = h

  action stepPromptScopeExfiltration = any { step, assembleChannelPromptUnchecked }

  run backendDmAndAntiExfilSafeTest =
    init
      .then(shareToDirectMessageAs(true, true))
      .then(readDirectMessageAs(true, true))
      .then(promoteScopeAs(true, true))
      .then(assembleChannelPromptSafe)
      .then(all {
        assert(h.dmShared),
        assert(h.entryScope == 1),
        assert(h.peerReads == 1),
        assert(safety),
        h' = h
      })

  run crossTenantCountermodelTest =
    init.then(shareToDirectMessageAs(true, true)).then(readCrossTenantUnchecked).then(checkUnsafe)

  run dmOutsiderCountermodelTest =
    init.then(shareToDirectMessageAs(true, true)).then(readDmOutsiderUnchecked).then(checkUnsafe)

  run promptExfiltrationCountermodelTest =
    init.then(assembleChannelPromptUnchecked).then(checkUnsafe)
}
