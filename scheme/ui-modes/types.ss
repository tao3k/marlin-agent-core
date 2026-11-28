;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; POO Contracts for Marlin's Emacs-authentic Major Mode, Minor Mode,
;;; Mode Hooks, Display-Buffer Policy, and AI-Native Polymode engine.
(import (only-in :clan/poo/object .cc .get object?)
        (only-in :clan/poo/type Bool Exactly Fun List Object Or Record String Symbol Type.)
        (only-in :std/list/list find))
(export DisplayActionContract
        SpanKindContract
        ModeHookPointContract
        ModeKeyBindingContract
        MajorModeContract
        MinorModeContract
        PolymodeInnerRuleContract
        PolymodeContract
        BufferModeRuleContract
        UiModesScopeContract)

(def IntContract
  (.cc Type. '.element? exact-integer?))

(def MaybeObjectContract
  (.cc Type. '.element? (lambda (x) (or (not x) (object? x)))))

(def VectorContract
  (.cc Type. '.element? vector?))

(def AnyContract
  (.cc Type. '.element? (lambda (_x) #t)))

(def DisplayActionContract
  (.cc Type. '.element?
       (lambda (x)
         (and (memq x '(reuse-window split-right split-below popup-bottom popup-right)) #t))))

(def SpanKindContract
  (.cc Type. '.element?
       (lambda (x)
         (and (memq x '(prose code-block diff-hunk tool-gate artifact conflict log-entry table-row))
              #t))))

(def ModeHookPointContract
  (.cc Type. '.element?
       (lambda (x)
         (and (memq x '(before-change-major-mode after-change-major-mode
                        enter-minor-mode exit-minor-mode
                        polymode-switch-span window-selection-change
                        buffer-create buffer-kill))
              #t))))

(def ModeKeyBindingContract
  (Record key: [String]
          command: [String]
          label: [String]
          exit: [Bool]))

(def MajorModeContract
  (Record id: [Symbol]
          title: [String]
          parent: [(Or Symbol (Exactly #f))]
          read-only: [Bool]
          display-action: [DisplayActionContract]
          dedicated: [Bool]
          local-keymap: [(List ModeKeyBindingContract)]
          default-minor-modes: [(List Symbol)]
          modeline-segments: [(List Symbol)]
          new-buffer-actions: [(List String)]
          enter-hooks: [(List Symbol)]
          exit-hooks: [(List Symbol)]))

(def MinorModeContract
  (Record id: [Symbol]
          title: [String]
          lighter: [String]
          priority: [IntContract]
          local-keymap: [(List ModeKeyBindingContract)]
          enter-hooks: [(List Symbol)]
          exit-hooks: [(List Symbol)]))

(def PolymodeInnerRuleContract
  (Record span-kind: [SpanKindContract]
          inner-mode: [Symbol]
          lighter: [String]))

(def PolymodeContract
  (Record id: [Symbol]
          host-mode: [Symbol]
          inner-rules: [(List PolymodeInnerRuleContract)]
          switch-span-hooks: [(List Symbol)]))

(def BufferModeRuleContract
  (Record prefix: [String]
          major-mode: [Symbol]
          creatable: [Bool]
          create-label: [String]
          create-action: [String]))

(def (check-unique-ids modes label)
  (let loop ((rest modes) (seen '()))
    (when (pair? rest)
      (let ((id (.get (car rest) id)))
        (when (memq id seen)
          (error (string-append "duplicate " label " id") id))
        (loop (cdr rest) (cons id seen))))))

(def (check-parent-acyclic major-modes mode-obj)
  (let loop ((cur (.get mode-obj parent)) (visited (list (.get mode-obj id))))
    (when cur
      (when (memq cur visited)
        (error "cyclic major-mode parent inheritance" cur visited))
      (let ((parent-obj (find (lambda (m) (eq? (.get m id) cur)) major-modes)))
        (unless parent-obj
          (error "major-mode references unknown parent" (.get mode-obj id) cur))
        (loop (.get parent-obj parent) (cons cur visited))))))

;; Hard POO instantiation contract: enforces acyclic Major Mode inheritance,
;; valid Minor Mode references, valid Polymode host/inner mode targets, and
;; valid buffer-to-mode routing rules.
(def (check-ui-modes-contracts scope)
  (let ((majors (.get scope major-modes))
        (minors (.get scope minor-modes))
        (inners (.get scope inner-modes))
        (polys (.get scope polymodes))
        (rules (.get scope buffer-mode-rules)))
    (check-unique-ids majors "major-mode")
    (check-unique-ids minors "minor-mode")
    (check-unique-ids inners "inner-mode")
    (check-unique-ids polys "polymode")
    (for-each
      (lambda (m)
        (check-parent-acyclic majors m)
        (for-each
          (lambda (mm)
            (unless (find (lambda (x) (eq? (.get x id) mm)) minors)
              (error "major-mode references unknown default minor-mode" (.get m id) mm)))
          (.get m default-minor-modes)))
      majors)
    (for-each
      (lambda (p)
        (unless (find (lambda (m) (eq? (.get m id) (.get p host-mode))) majors)
          (error "polymode references unknown host-mode" (.get p id) (.get p host-mode)))
        (for-each
          (lambda (r)
            (let ((im (.get r inner-mode)))
              (unless (or (find (lambda (m) (eq? (.get m id) im)) inners)
                          (find (lambda (m) (eq? (.get m id) im)) majors))
                (error "polymode references unknown inner-mode" (.get p id) im))))
          (.get p inner-rules)))
      polys)
    (for-each
      (lambda (rule)
        (unless (find (lambda (m) (eq? (.get m id) (.get rule major-mode))) majors)
          (error "buffer-mode-rule references unknown major-mode"
                 (.get rule prefix) (.get rule major-mode))))
      rules))
  scope)

(def UiModesScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-modes)]
               major-modes: [(List MajorModeContract)]
               minor-modes: [(List MinorModeContract)]
               inner-modes: [(List MajorModeContract)]
               polymodes: [(List PolymodeContract)]
               buffer-mode-rules: [(List BufferModeRuleContract)]
               parent-chain: [(Fun (List Symbol) <- Object Symbol)]
               resolve-buffer-mode: [(Fun Symbol <- Object String)]
               resolve-polymode-inner: [(Fun (Or Symbol (Exactly #f)) <- Object Symbol SpanKindContract)]
               make-buffer-state: [(Fun Object <- Object String)]
               effective-stack: [(Fun Object <- Object Object SpanKindContract)]
               lookup-key: [(Fun (Or ModeKeyBindingContract (Exactly #f)) <- Object Object SpanKindContract String)]
               switch-span: [(Fun Object <- Object MaybeObjectContract Object SpanKindContract)]
               toggle-minor-mode: [(Fun Object <- Object MaybeObjectContract Object Symbol)]
               project-grpc-registry: [(Fun VectorContract <- AnyContract)])
       identity: 'marlin/ui-modes
       .instantiate: check-ui-modes-contracts))
