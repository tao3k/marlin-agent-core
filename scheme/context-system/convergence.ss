;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure functional Yuanxu Design 165 convergence catalog for Marlin modules:
;;; Two-tier interface slot-specs, POO `.o` dispatch tables, plural global/module
;;; profiles, GovernanceFloor non-escalation, Band 1 implied feature bridging,
;;; and zero-recompile runtime overlays.

(import (only-in :clan/poo/object .def .get .o .ref .slot?)
        (only-in :tao3k/yuanxu/module-system/convergence/func
                 make-slot-spec
                 make-governance-envelope
                 governance-envelope-meet
                 by-tag
                 make-feature-decl
                 make-module-decl
                 make-global-profile
                 converge-assembly
                 converge-runtime-overlay))

(export MarlinLocalDevProfile.
        MarlinAirgappedProfile.
        MarlinDetourBlockerProfile.
        MarlinStrictSoDProfile.
        marlin-design165-catalog
        marlin-converge-assembly
        marlin-converge-runtime-overlay
        marlin-make-global-profile
        marlin-by-tag
        marlin-governance-envelope)

(def MarlinLocalDevProfile.
  (.o bind-these: #f
      profile-id: 'marlin-local-dev
      no-live-llm: #f
      hooks: '(local-dev-trace)))

(def MarlinAirgappedProfile.
  (.o bind-these: #f
      profile-id: 'marlin-airgapped
      no-live-llm: #t
      governance-envelope: (make-governance-envelope 1 #f #t)))

(def MarlinDetourBlockerProfile.
  (.o bind-these: #f
      profile-id: 'marlin-detour-blocker
      auto-archive: #f
      hooks: '(detour-blocker-guard)))

(def MarlinStrictSoDProfile.
  (.o bind-these: #f
      profile-id: 'marlin-strict-sod
      require-independent-reviewer: #t
      hooks: '(sod-audit)))

(def (marlin-design165-catalog)
  (let* ((env-mod
          (make-module-decl
           'env
           tags: '(environment)
           slot-specs: (list (make-slot-spec 'mode
                                             stage: 'assembly
                                             combiner: 'exclusive
                                             default: 'local-dev
                                             allowed: '(local-dev ci-hermetic airgapped staging)))))
         (durable-mod
          (make-module-decl
           'durable
           tags: '(engine durable)
           slot-specs: (list (make-slot-spec 'detour-state
                                             stage: 'runtime
                                             combiner: 'exclusive
                                             default: 'mainline
                                             allowed: '(mainline blocker-active side-quest-active)))
           features: '()))
         (artifacts-mod
          (make-module-decl
           'artifacts
           tags: '(gitops storage governed)
           slot-specs: (list (make-slot-spec 'require-independent-reviewer
                                             stage: 'assembly
                                             combiner: 'exclusive
                                             default: #t)
                             (make-slot-spec 'hooks
                                             stage: 'assembly
                                             combiner: 'union
                                             default: '(gitops-org-contract)))))
         (ui-mod
          (make-module-decl
           'ui-workbench
           tags: '(ui)
           slot-specs: (list (make-slot-spec 'tab-providers
                                             stage: 'assembly
                                             combiner: 'union
                                             default: '(index-tab))
                             (make-slot-spec 'disclosure-default
                                             stage: 'assembly
                                             combiner: 'exclusive
                                             default: 'L1
                                             allowed: '(L0 L1 L2 L3)))))
         (context-durable-feat
          (make-feature-decl
           'context-system 'durable
           enabled-fn: (lambda (sys) (.get sys durable enabled?))
           contributions: (.o bind-these: #f
                              ui-workbench: (.o bind-these: #f
                                                tab-providers: '(durable-task-tabs)))))
         (context-mod
          (make-module-decl
           'context-system
           tags: '(context org-backed governed)
           slot-specs: (list (make-slot-spec 'no-live-llm
                                             stage: 'assembly
                                             combiner: 'exclusive
                                             default: #f)
                             (make-slot-spec 'auto-archive
                                             stage: 'assembly
                                             combiner: 'exclusive
                                             default: #t)
                             (make-slot-spec 'focused-tab-archive
                                             stage: 'assembly
                                             combiner: 'exclusive
                                             default: 'on-blur
                                             allowed: '(immediate on-blur manual))
                             (make-slot-spec 'hooks
                                             stage: 'assembly
                                             combiner: 'union
                                             default: '(core-org))
                             (make-slot-spec 'governance-envelope
                                             stage: 'assembly
                                             combiner: 'meet
                                             default: (make-governance-envelope 2 #t #t)
                                             meet: governance-envelope-meet))
           features: (list context-durable-feat)
           profiles: (.o bind-these: #f
                         env.mode: (.o bind-these: #f
                                       ci-hermetic: MarlinAirgappedProfile.
                                       airgapped: MarlinAirgappedProfile.)
                         durable.detour-state: (.o bind-these: #f
                                                   blocker-active: MarlinDetourBlockerProfile.))
           governance-floor: (.o bind-these: #f
                                 governance-envelope: (make-governance-envelope 2 #t #t)))))
    (list env-mod durable-mod artifacts-mod ui-mod context-mod)))

(def (marlin-converge-assembly modules
                               global-profiles: (global-profiles '())
                               module-profiles: (module-profiles '())
                               explicit-flags: (explicit-flags '())
                               catalog: (catalog #f))
  (converge-assembly (or catalog (marlin-design165-catalog))
                     modules
                     global-profiles: global-profiles
                     module-profiles: module-profiles
                     explicit-flags: explicit-flags))

(def (marlin-converge-runtime-overlay assembly-result runtime-updates
                                      catalog: (catalog #f))
  (converge-runtime-overlay (or catalog (marlin-design165-catalog))
                            assembly-result
                            runtime-updates))

(def marlin-make-global-profile make-global-profile)
(def marlin-by-tag by-tag)
(def marlin-governance-envelope make-governance-envelope)
