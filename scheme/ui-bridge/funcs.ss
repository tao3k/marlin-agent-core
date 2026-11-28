;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Unified Scheme Bridge Hub (`scheme/ui-bridge/funcs.ss`):
;;; ONE modular Scheme package dispatcher crossing C-ABI into Rust's single
;;; `UnifiedSchemeBridge`. Adding a new Emacs/Scheme package requires only
;;; registering a module here — ZERO new Rust bridge files or C-ABI symbols.
(import (only-in :clan/poo/object .get .o)
        "./objects.ss"
        "./record.ss"
        "./org-module.ss"
        "./markdown-module.ss")
(export DefaultBridgeModules
        marlin-scheme-bridge-list-namespaces
        marlin-scheme-bridge-register!
        marlin-scheme-bridge-dispatch)

;; Auto-detect Org Mode vs Markdown by file extension or leading Org directives.
(def (org-document? path payload)
  (let ((lo (str-downcase (str-trim path))))
    (cond
      ((str-suffix? ".org" lo) #t)
      ((or (str-suffix? ".md" lo) (str-suffix? ".markdown" lo)) #f)
      (else
       (let ((t (str-trim payload)))
         (or (string-prefix? "#+" t)
             (string-prefix? "* " t)
             (string-prefix? "** " t)))))))

(def (doc-handle-query verb arg payload)
  (if (org-document? arg payload)
    ((.get OrgBridgeModule handle-query) verb arg payload)
    ((.get MarkdownBridgeModule handle-query) verb arg payload)))

(def DocBridgeModule
  (.o namespace: "doc"
      owner-id: "marlin.scheme.doc"
      description: "Unified Org Mode & Markdown Document IR router (auto-dispatches .org / .md)"
      handle-query: doc-handle-query))

(def DefaultBridgeModules
  (list OrgBridgeModule
        MarkdownBridgeModule
        DocBridgeModule))

(def *bridge-registry*
  (let ((ht (make-hash-table)))
    (for-each (lambda (m) (hash-put! ht (.get m namespace) m)) DefaultBridgeModules)
    ht))

;; Register (or replace) a Scheme package bridge module dynamically at runtime.
(def (marlin-scheme-bridge-register! mod)
  (hash-put! *bridge-registry* (.get mod namespace) mod)
  (.get mod namespace))

;; C-ABI entrypoint #1: returns `#("org" "markdown" "doc" ...)` of all registered
;; Scheme bridge namespaces so Rust auto-wires every module with zero hardcoding.
(def (marlin-scheme-bridge-list-namespaces _unused)
  (list->vector (hash-keys *bridge-registry*)))

;; C-ABI entrypoint #2: unified native vector dispatch for ALL Scheme packages.
;; Input:  #(namespace verb arg payload)
;; Output: #(#(id kind title summary source-path start-line end-line #(#(k v) ...)) ...)
(def (marlin-scheme-bridge-dispatch request)
  (unless (and (vector? request) (= (vector-length request) 4))
    (error "marlin-scheme-bridge-dispatch: expected 4-element vector #(ns verb arg payload)"))
  (let* ((ns (vector-ref request 0))
         (verb (vector-ref request 1))
         (arg (vector-ref request 2))
         (payload (vector-ref request 3))
         (mod (hash-get *bridge-registry* ns)))
    (unless mod
      (error "marlin-scheme-bridge-dispatch: unknown Scheme bridge namespace" ns))
    (let ((records ((.get mod handle-query) verb arg payload)))
      (list->vector records))))
