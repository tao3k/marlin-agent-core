;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure functional Major Mode inheritance, Minor Mode stacking, Mode Hooks,
;;; modern span-driven Polymode resolution, Emacs-order keymap lookup, and
;;; native C-ABI vector projection for Rust/Go.
(import (only-in :clan/poo/object .cc .get .o)
        (only-in :std/list/list filter find)
        "./objects.ss"
        "./types.ss")
(export marlin-modes-parent-chain
        marlin-modes-resolve-buffer-mode
        marlin-modes-resolve-polymode-inner
        marlin-modes-make-buffer-state
        marlin-modes-effective-stack
        marlin-modes-lookup-key
        marlin-modes-switch-span
        marlin-modes-toggle-minor-mode
        marlin-modes-project-grpc-registry)

(def (string-prefix? prefix str)
  (let ((plen (string-length prefix))
        (slen (string-length str)))
    (and (<= plen slen)
         (string=? (substring str 0 plen) prefix))))

(def (find-mode-by-id modes id)
  (find (lambda (m) (eq? (.get m id) id)) modes))

(def (find-major-or-inner registry id)
  (or (find-mode-by-id (.get registry major-modes) id)
      (find-mode-by-id (.get registry inner-modes) id)))

;; Walk Major Mode parent inheritance chain: e.g.
;;   magit-status-mode -> (magit-status-mode magit-mode marlin-special-mode fundamental-mode)
(def (marlin-modes-parent-chain registry mode-id)
  (let loop ((cur mode-id) (acc '()))
    (if (not cur)
      (reverse acc)
      (let ((obj (find-major-or-inner registry cur)))
        (if (not obj)
          (reverse (cons cur acc))
          (loop (.get obj parent) (cons cur acc)))))))

;; Resolve a buffer ID (e.g. "*chat:auth-fix*", "*magit*") to its Major Mode symbol.
(def (marlin-modes-resolve-buffer-mode registry buffer-id)
  (let ((rule (find (lambda (r) (string-prefix? (.get r prefix) buffer-id))
                    (.get registry buffer-mode-rules))))
    (if rule
      (.get rule major-mode)
      'fundamental-mode)))

;; Resolve Polymode inner mode for (host-mode, span-kind), returning inner-mode symbol or #f.
(def (marlin-modes-resolve-polymode-inner registry host-mode-id span-kind)
  (let ((poly (find (lambda (p) (eq? (.get p host-mode) host-mode-id))
                    (.get registry polymodes))))
    (if (not poly)
      #f
      (let ((rule (find (lambda (r) (eq? (.get r span-kind) span-kind))
                        (.get poly inner-rules))))
        (if rule (.get rule inner-mode) #f)))))

(def (polymode-lighter-for registry host-mode-id span-kind)
  (let ((poly (find (lambda (p) (eq? (.get p host-mode) host-mode-id))
                    (.get registry polymodes))))
    (if (not poly)
      ""
      (let ((rule (find (lambda (r) (eq? (.get r span-kind) span-kind))
                        (.get poly inner-rules))))
        (if rule (.get rule lighter) "")))))

;; Construct initial pure Buffer Mode state for a buffer ID.
(def (marlin-modes-make-buffer-state registry buf-id)
  (let* ((major-id (marlin-modes-resolve-buffer-mode registry buf-id))
         (major-obj (find-major-or-inner registry major-id))
         (minors (if major-obj (.get major-obj default-minor-modes) '(meow-normal-mode)))
         (hooks (if major-obj (.get major-obj enter-hooks) '())))
    (.o buffer-id: buf-id
        major-mode: major-id
        minor-modes: minors
        focused-span: 'prose
        inner-mode: #f
        hook-log: hooks)))

(def (insert-by-priority elem sorted)
  (cond
    ((null? sorted) (list elem))
    ((> (.get elem priority) (.get (car sorted) priority))
     (cons elem sorted))
    (else (cons (car sorted) (insert-by-priority elem (cdr sorted))))))

(def (sort-minor-modes registry minor-ids)
  (let loop ((rest minor-ids) (acc '()))
    (if (null? rest)
      acc
      (let ((obj (find-mode-by-id (.get registry minor-modes) (car rest))))
        (if obj
          (loop (cdr rest) (insert-by-priority obj acc))
          (loop (cdr rest) acc))))))

;; Compute the Effective Mode Stack for a buffer at a focused Polymode span.
(def (marlin-modes-effective-stack registry buf-state focused-span-kind)
  (let* ((host-id (.get buf-state major-mode))
         (host-obj (find-major-or-inner registry host-id))
         (inner-id (marlin-modes-resolve-polymode-inner registry host-id focused-span-kind))
         (inner-obj (and inner-id (find-major-or-inner registry inner-id)))
         (minor-objs (sort-minor-modes registry (.get buf-state minor-modes)))
         (parent-ids (marlin-modes-parent-chain registry host-id))
         (inner-lighter (polymode-lighter-for registry host-id focused-span-kind))
         (base-title (if host-obj (.get host-obj title) (symbol->string host-id)))
         (title-str (if (and inner-id (not (string=? inner-lighter "")))
                      (string-append base-title "[" inner-lighter "]")
                      base-title)))
    (.o host-mode: host-id
        inner-mode: inner-id
        minor-modes: (map (lambda (m) (.get m id)) minor-objs)
        parent-chain: parent-ids
        display-title: title-str
        display-action: (if host-obj (.get host-obj display-action) 'reuse-window)
        read-only: (if host-obj (.get host-obj read-only) #f)
        modeline-segments: (if host-obj (.get host-obj modeline-segments) '())
        inner-keymap: (if inner-obj (.get inner-obj local-keymap) '()))))

;; Exact Emacs keymap lookup precedence:
;;   1. Active minor-mode maps (highest priority first)
;;   2. Polymode inner-mode map (when focused inside a matching span)
;;   3. Major Mode map walking parent-chain (child overrides parent)
;;   4. Returns #f when unbound locally -> caller falls through to global map.
(def (find-key-in-bindings bindings key)
  (find (lambda (b) (string=? (.get b key) key)) bindings))

(def (marlin-modes-lookup-key registry buf-state focused-span-kind key)
  (let* ((host-id (.get buf-state major-mode))
         (minor-objs (sort-minor-modes registry (.get buf-state minor-modes)))
         (inner-id (marlin-modes-resolve-polymode-inner registry host-id focused-span-kind))
         (parent-ids (marlin-modes-parent-chain registry host-id)))
    ;; Tier 1: Active Minor Modes (priority order)
    (or (let loop ((ms minor-objs))
          (if (null? ms)
            #f
            (or (find-key-in-bindings (.get (car ms) local-keymap) key)
                (loop (cdr ms)))))
        ;; Tier 2: Polymode Inner Mode (if cursor is in an inner span)
        (and inner-id
             (let ((inner-chain (marlin-modes-parent-chain registry inner-id)))
               (let loop ((ids inner-chain))
                 (if (null? ids)
                   #f
                   (let ((obj (find-major-or-inner registry (car ids))))
                     (or (and obj (find-key-in-bindings (.get obj local-keymap) key))
                         (loop (cdr ids))))))))
        ;; Tier 3: Host Major Mode -> Parent Chain
        (let loop ((ids parent-ids))
          (if (null? ids)
            #f
            (let ((obj (find-major-or-inner registry (car ids))))
              (or (and obj (find-key-in-bindings (.get obj local-keymap) key))
                  (loop (cdr ids)))))))))

;; Pure Polymode span transition: updates focused-span & inner-mode, firing
;; polymode-switch-span hooks ONLY when the effective inner mode changes.
(def (marlin-modes-switch-span registry _hook-registry buf-state next-span-kind)
  (let* ((host-id (.get buf-state major-mode))
         (prev-inner (.get buf-state inner-mode))
         (next-inner (marlin-modes-resolve-polymode-inner registry host-id next-span-kind))
         (changed? (not (eq? prev-inner next-inner)))
         (poly (find (lambda (p) (eq? (.get p host-mode) host-id))
                     (.get registry polymodes)))
         (fired (if (and changed? poly) (.get poly switch-span-hooks) '())))
    (.cc buf-state
         focused-span: next-span-kind
         inner-mode: next-inner
         hook-log: (append (.get buf-state hook-log) fired))))

;; Pure Minor Mode toggle: enables or disables a minor mode and records hooks.
(def (marlin-modes-toggle-minor-mode registry _hook-registry buf-state minor-id)
  (let* ((active (.get buf-state minor-modes))
         (present? (and (memq minor-id active) #t))
         (obj (find-mode-by-id (.get registry minor-modes) minor-id))
         (next-minors (if present?
                        (filter (lambda (id) (not (eq? id minor-id))) active)
                        (cons minor-id active)))
         (fired (cond
                  ((not obj) '())
                  (present? (.get obj exit-hooks))
                  (else (.get obj enter-hooks)))))
    (.cc buf-state
         minor-modes: next-minors
         hook-log: (append (.get buf-state hook-log) fired))))

(def (project-bindings bindings)
  (list->vector
    (map (lambda (b)
           (vector (.get b key) (.get b command) (.get b label) (.get b exit)))
         bindings)))

(def (project-mode-entry m)
  (vector (symbol->string (.get m id))
          (.get m title)
          (if (.get m parent) (symbol->string (.get m parent)) "")
          (.get m read-only)
          (symbol->string (.get m display-action))
          (.get m dedicated)
          (project-bindings (.get m local-keymap))
          (list->vector (map symbol->string (.get m default-minor-modes)))
          (list->vector (map symbol->string (.get m modeline-segments)))
          (list->vector (.get m new-buffer-actions))))

;; Native C-ABI FFI projection for Rust `marlin-gerbil-scheme` -> gRPC -> Go:
;; returns pure Scheme vector #(major-modes minor-modes inner-modes polymodes buffer-rules)
;; with zero text serialization.
(def (marlin-modes-project-grpc-registry _input)
  (vector
    (list->vector (map project-mode-entry DefaultMajorModes))
    (list->vector
      (map (lambda (m)
             (vector (symbol->string (.get m id))
                     (.get m title)
                     (.get m lighter)
                     (.get m priority)
                     (project-bindings (.get m local-keymap))))
           DefaultMinorModes))
    (list->vector (map project-mode-entry DefaultInnerModes))
    (list->vector
      (map (lambda (p)
             (vector (symbol->string (.get p id))
                     (symbol->string (.get p host-mode))
                     (list->vector
                       (map (lambda (r)
                              (vector (symbol->string (.get r span-kind))
                                      (symbol->string (.get r inner-mode))
                                      (.get r lighter)))
                            (.get p inner-rules)))))
           DefaultPolymodes))
    (list->vector
      (map (lambda (r)
             (vector (.get r prefix)
                     (symbol->string (.get r major-mode))
                     (.get r creatable)
                     (.get r create-label)
                     (.get r create-action)))
           DefaultBufferModeRules))))
