// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
// @quint-extends "@deterministic-engineering/quint-models/threat/zero-trust/multi-tenant-isolation/MultiTenantIsolation"
// @countermodel tenant-admin-sod-distinct-actor step=stepTenantAdminSelfApproval max-steps=6 itf="h.sodSelfApprovalLeak == true"
// @countermodel tenant-admin-veto-precedence step=stepTenantAdminVetoOverride max-steps=6 itf="h.vetoOverrideLeak == true"

// Marlin Rust backend MultiTenantIsolation overlay composing tenant-resource pinning
// and offline outbox isolation with dual-control Separation of Duties (SoD) and veto precedence.
module marlin_backend_multi_tenant_sod_overlay {
  type State = {
    adminProposer: int,
    adminApprover: int,
    adminVetoed: bool,
    adminExecuted: bool,
    sodSelfApprovalLeak: bool,
    vetoOverrideLeak: bool
  }

  action init = h' = {
    adminProposer: 0,
    adminApprover: 0,
    adminVetoed: false,
    adminExecuted: false,
    sodSelfApprovalLeak: false,
    vetoOverrideLeak: false
  }

  action proposeTenantAdminAs(actor: int): bool =
    if (h.adminProposer == 0 and actor > 0)
      h' = h.with("adminProposer", actor)
    else
      h' = h

  action proposeTenantAdmin = {
    nondet actor = Set(1, 2).oneOf()
    proposeTenantAdminAs(actor)
  }

  action approveTenantAdminAs(actor: int): bool =
    if (h.adminProposer > 0 and h.adminApprover == 0 and actor != h.adminProposer and not(h.adminVetoed))
      h' = h
        .with("adminApprover", actor)
        .with("sodSelfApprovalLeak", h.sodSelfApprovalLeak or actor == h.adminProposer)
    else
      h' = h

  action approveTenantAdmin = {
    nondet actor = Set(1, 2).oneOf()
    approveTenantAdminAs(actor)
  }

  action vetoTenantAdmin =
    if (not(h.adminExecuted))
      h' = h.with("adminVetoed", true)
    else
      h' = h

  action executeTenantAdmin =
    if (h.adminProposer > 0 and h.adminApprover > 0 and h.adminApprover != h.adminProposer and not(h.adminVetoed) and not(h.adminExecuted))
      h' = h
        .with("adminExecuted", true)
        .with("sodSelfApprovalLeak", h.sodSelfApprovalLeak or h.adminApprover == h.adminProposer)
        .with("vetoOverrideLeak", h.vetoOverrideLeak or h.adminVetoed)
    else
      h' = h

  action step = any {
    proposeTenantAdmin,
    approveTenantAdmin,
    vetoTenantAdmin,
    executeTenantAdmin
  }

  val safety = not(h.sodSelfApprovalLeak) and not(h.vetoOverrideLeak)

  action approveTenantAdminSelfAllowed(actor: int): bool =
    if (h.adminProposer > 0 and h.adminApprover == 0 and not(h.adminVetoed))
      h' = h
        .with("adminApprover", actor)
        .with("sodSelfApprovalLeak", h.sodSelfApprovalLeak or actor == h.adminProposer)
    else
      h' = h

  action stepTenantAdminSelfApproval = any {
    step,
    {
      nondet actor = Set(1, 2).oneOf()
      approveTenantAdminSelfAllowed(actor)
    }
  }

  action executeTenantAdminOverVeto =
    if (h.adminProposer > 0 and h.adminApprover > 0 and h.adminApprover != h.adminProposer and not(h.adminExecuted))
      h' = h
        .with("adminExecuted", true)
        .with("vetoOverrideLeak", h.vetoOverrideLeak or h.adminVetoed)
    else
      h' = h

  action stepTenantAdminVetoOverride = any { step, executeTenantAdminOverVeto }

  run tenantAdminDualControlSafeTest =
    init
      .then(proposeTenantAdminAs(1))
      .then(approveTenantAdminAs(1))
      .then(approveTenantAdminAs(2))
      .then(executeTenantAdmin)
      .then(all {
        assert(h.adminExecuted),
        assert(not(h.sodSelfApprovalLeak)),
        assert(not(h.vetoOverrideLeak)),
        assert(safety),
        h' = h
      })

  run tenantAdminSelfApprovalCountermodelTest =
    init.then(proposeTenantAdminAs(1)).then(approveTenantAdminSelfAllowed(1)).then(checkUnsafe)

  run tenantAdminVetoOverrideCountermodelTest =
    init.then(proposeTenantAdminAs(1)).then(approveTenantAdminAs(2)).then(vetoTenantAdmin).then(executeTenantAdminOverVeto).then(checkUnsafe)
}
