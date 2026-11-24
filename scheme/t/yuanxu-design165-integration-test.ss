;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Regression tests for Yuanxu Design 165 convergence and lazy activation-receipts
;;; wired into Marlin (`context-system/convergence.ss` + `marlin-agent-core/init`).

(import :std/test
        (only-in :std/list/list find)
        (only-in :clan/poo/object .get .o .ref .slot?)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports)
        (only-in :marlin-agent-core/context-system/convergence
                 MarlinAirgappedProfile.
                 MarlinDetourBlockerProfile.
                 marlin-converge-assembly
                 marlin-converge-runtime-overlay
                 marlin-make-global-profile
                 marlin-by-tag
                 marlin-governance-envelope))

(export yuanxu-design165-integration-test)

(def yuanxu-design165-integration-test
  (test-suite "Marlin Yuanxu Design 165 Convergence and Activation Receipts"

    (test-case "Live Marlin init.exports publishes lazy activation-receipts with context-system features"
      (check (.slot? marlin-agent-core.init.exports 'activation-receipts) => #t)
      (let ((receipts (.ref marlin-agent-core.init.exports 'activation-receipts)))
        (check (list? receipts) => #t)
        (let ((ctx-dur (find (lambda (r) (eq? (.ref r 'identity) 'marlin-agent-core/context-system/interface/durable)) receipts))
              (ctx-cau (find (lambda (r) (eq? (.ref r 'identity) 'marlin-agent-core/context-system/interface/causal)) receipts))
              (git-wt (find (lambda (r) (eq? (.ref r 'identity) 'marlin-agent-core/gitops/interface/worktrunk)) receipts)))
          (check (and ctx-dur #t) => #t)
          (check (and ctx-cau #t) => #t)
          (check (and git-wt #t) => #t)
          (check (.ref ctx-dur 'activation) => 'explicit)
          (check (.ref ctx-cau 'activation) => 'explicit)
          (check (.ref git-wt 'activation) => 'explicit))))

    (test-case "Band 1 implied bridge auto-activates context-system/durable and contributes UI tab provider"
      ;; Without `durable`, bridge stays unloaded (INV-1 / INV-6)
      (let ((r-alone (marlin-converge-assembly '(env context-system artifacts ui-workbench))))
        (check (.ref r-alone 'status) => 'ready)
        (check (.ref r-alone 'active-features) => '())
        (check (.get (.ref r-alone 'effective-configs) ui-workbench tab-providers)
               => '(index-tab)))
      ;; With `durable`, bridge auto-activates in Band 1 and extends `ui-workbench.tab-providers`
      (let ((r-both (marlin-converge-assembly '(env durable context-system artifacts ui-workbench))))
        (check (.ref r-both 'status) => 'ready)
        (check (.ref r-both 'active-features) => '(context-system/durable))
        (check (.get (.ref r-both 'effective-configs) ui-workbench tab-providers)
               => '(index-tab durable-task-tabs)))
      ;; Explicit Band 4b `-durable` veto dominates Band 1 implied bridge (INV-3)
      (let ((r-veto (marlin-converge-assembly '(env durable context-system artifacts ui-workbench)
                                              explicit-flags: '((context-system/durable . #f)))))
        (check (.ref r-veto 'status) => 'ready)
        (check (.ref r-veto 'active-features) => '())
        (check (.get (.ref r-veto 'effective-configs) ui-workbench tab-providers)
               => '(index-tab))))

    (test-case "Plural Global Profiles, by-tag targeting, GovernanceFloor, and zero-recompile runtime overlay"
      (let* ((ci-profile (marlin-make-global-profile
                          'marlin-ci '(env) (.o bind-these: #f mode: 'ci-hermetic)))
             (tag-profile (marlin-make-global-profile
                           'governed-hooks (list (marlin-by-tag 'governed))
                           (.o bind-these: #f hooks: '(policy-audit-hook))))
             (r-asm (marlin-converge-assembly
                     '(env durable context-system artifacts ui-workbench)
                     global-profiles: (list ci-profile tag-profile))))
        (check (.ref r-asm 'status) => 'ready)
        (check (.get (.ref r-asm 'effective-configs) context-system no-live-llm) => #t)
        (check (.get (.ref r-asm 'effective-configs) context-system hooks)
               => '(core-org policy-audit-hook))
        (check (.get (.ref r-asm 'effective-configs) artifacts hooks)
               => '(gitops-org-contract policy-audit-hook))
        (check (.ref (.get (.ref r-asm 'effective-configs) context-system governance-envelope)
                     'allow-network)
               => #f)
        ;; Zero-recompile runtime overlay on `durable.detour-state = 'blocker-active` (INV-6)
        (let ((r-rt (marlin-converge-runtime-overlay
                     r-asm
                     (list (cons 'durable (.o bind-these: #f detour-state: 'blocker-active))))))
          (check (.ref r-rt 'status) => 'ready)
          (check (.ref r-rt 'compiled-closure) => (.ref r-asm 'compiled-closure))
          (check (.get (.ref r-rt 'effective-configs) context-system auto-archive) => #f)
          (check (.get (.ref r-rt 'effective-configs) context-system hooks)
                 => '(core-org policy-audit-hook detour-blocker-guard)))))

    (test-case "Fail-closed peer exclusive conflict (INV-4) and GovernanceFloor non-escalation (INV-5)"
      (let* ((gp1 (marlin-make-global-profile 'p1 '(context-system)
                                              (.o bind-these: #f focused-tab-archive: 'on-blur)))
             (gp2 (marlin-make-global-profile 'p2 '(context-system)
                                              (.o bind-these: #f focused-tab-archive: 'manual)))
             (r-conf (marlin-converge-assembly '(env context-system)
                                               global-profiles: (list gp1 gp2))))
        (check (.ref r-conf 'status) => 'rejected)
        (check (.ref (car (.ref r-conf 'diagnostics)) 'code) => 'peer-exclusive-conflict))
      (let* ((escalate-prof (.o bind-these: #f
                                profile-id: 'escalate-trust
                                governance-envelope: (marlin-governance-envelope 99 #t #t)))
             (r-floor (marlin-converge-assembly
                       '(env context-system)
                       module-profiles: (list (cons 'context-system (list escalate-prof))))))
        (check (.ref r-floor 'status) => 'ready)
        (check (.ref (.get (.ref r-floor 'effective-configs) context-system governance-envelope)
                     'trust-ceiling)
               => 2)))))
