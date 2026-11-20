;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :marlin-agent-core/c-abi/roots marlin-rooted-call)
        (only-in :marlin-agent-core/context-system/worktree prepare-worktree-context deliver-worktree-context)
        (for-syntax :gerbil/expander
          (only-in :clan/poo/object .call)
          (only-in "./build.ss" Build)))

;; Compile the capability from the admitted GitOps Feature. The host checks
;; executable availability when the user requests a switch.
(defsyntax (selected-worktrunk stx)
  (syntax-case stx ()
    ((keyword)
     (with-syntax ((provider (datum->syntax #'keyword (.call Build .worktree-provider))))
       #'(quote provider)))))

(def (worktree-switch-provider _) (selected-worktrunk))

(begin-foreign
  (c-define (worktree-prepare root) (int64) int64
    "marlin_worktree_context_prepare" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/context-system/worktree#prepare-worktree-context))
  (c-define (worktree-deliver root) (int64) int64
    "marlin_worktree_context_deliver" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/context-system/worktree#deliver-worktree-context))
  (c-define (worktree-provider root) (int64) int64
    "marlin_worktree_provider" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/worktree-context#worktree-switch-provider)))
