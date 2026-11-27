;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Typed contracts for Marlin's AI-Native Magit Scheme plane.
;;; Enforces mandatory dual keybindings (Doom SPC leader <-> native Emacs C-*/M-*)
;;; and honest evidence-chain risk typing (Aitia gRPC + Formal + Human/LLM).
(import (only-in :clan/poo/object .cc .get)
        (only-in :clan/poo/type Any Bool Exactly List Object Record Symbol Type.))
(export MagitRiskLevel
        MagitAuditState
        MagitEvidenceSource
        MagitSectionKind
        MagitBackend
        EmacsKeyContract
        DoomLeaderBindingContract
        MagitHunkContract
        MagitAitiaContextContract
        UiMagitScopeContract)

(def (symbol-in? allowed)
  (lambda (v) (and (symbol? v) (memq v allowed) #t)))

(def NonEmptyStringContract
  (.cc Type. '.element? (lambda (v) (and (string? v) (> (string-length v) 0)))))

(def IntContract
  (.cc Type. '.element? exact-integer?))

(def ProcedureContract
  (.cc Type. '.element? procedure?))

(def (native-emacs-key-string? s)
  (and (string? s)
       (>= (string-length s) 3)
       (or (string-prefix? "C-" s)
           (string-prefix? "M-" s))))

(def EmacsKeyContract
  (.cc Type. '.element? native-emacs-key-string?))

(def MagitRiskLevel
  (.cc Type. '.element? (symbol-in? '(high medium low info unassessed))))

(def MagitAuditState
  (.cc Type. '.element? (symbol-in? '(unreviewed audited flagged disputed))))

(def MagitEvidenceSource
  (.cc Type. '.element? (symbol-in? '(aitia-grpc formal-model human-audit llm-audit structural))))

(def MagitSectionKind
  (.cc Type. '.element?
       (symbol-in? '(head conflicts unstaged staged untracked stashes unpushed unpulled log oplog))))

(def MagitBackend
  (.cc Type. '.element? (symbol-in? '(jj-colocated git))))

(def DoomLeaderBindingContract
  (Record
    prefix: [NonEmptyStringContract]
    key: [NonEmptyStringContract]
    emacs-key: [EmacsKeyContract]
    label: [NonEmptyStringContract]
    column: [NonEmptyStringContract]
    action: [NonEmptyStringContract]
    exit: [Bool]))

(def MagitHunkContract
  (Record
    hunk-id: [NonEmptyStringContract]
    file-path: [NonEmptyStringContract]
    header: [NonEmptyStringContract]
    risk: [MagitRiskLevel]
    concern: [Symbol]
    risk-reason: [NonEmptyStringContract]
    added: [IntContract]
    removed: [IntContract]
    preview: [Any]))

(def MagitAitiaContextContract
  (Record
    evidence-id: [NonEmptyStringContract]
    backend: [MagitBackend]
    branch: [NonEmptyStringContract]
    change-id: [Any]
    commit-sha: [Any]
    op-id: [Any]))

(def (check-magit-dual-bindings scope)
  (for-each
    (lambda (b)
      (let ((pref (.get b prefix))
            (k (.get b key))
            (ek (.get b emacs-key)))
        (unless (native-emacs-key-string? ek)
          (error "Doom SPC g binding must declare a native Emacs (C-*/M-*) chord" pref k ek))))
    (.get scope doom-leader-bindings))
  scope)

(def UiMagitScopeContract
  (.cc (Record
         identity: [(Exactly 'marlin/ui-magit)]
         doom-leader-bindings: [(List DoomLeaderBindingContract)]
         magit-transients: [(List Object)]
         make-state: [ProcedureContract]
         toggle-fold: [ProcedureContract]
         set-visibility-level: [ProcedureContract]
         step-cursor: [ProcedureContract]
         mark-hunk-audit: [ProcedureContract]
         audit-progress: [ProcedureContract]
         sort-hunks-by-risk: [ProcedureContract]
         fuse-evidence-risk: [ProcedureContract]
         build-ai-audit-prompt: [ProcedureContract]
         build-ai-commit-prompt: [ProcedureContract]
         transient-for: [ProcedureContract])
       identity: 'marlin/ui-magit
       .instantiate: check-magit-dual-bindings))
