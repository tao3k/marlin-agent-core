;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Opt-in `+ast-patch` Yuanxu Feature of `scheme/repair`.
;;;
;;; Bridges Search DSL `entity_locator` & `ast-style` (`propose-ast-patch` /
;;; `auto-elevate-local`) findings into typed, fail-closed AST patch requests
;;; aligned 1:1 with ASP `POST /v1/ast/patch` (`AspHttpAstPatchOp`) and
;;; zero-JSON rooted C-ABI vector projection.
(import (only-in :clan/poo/object .o .ref .slot? seal-object!)
        (only-in :std/string/misc string-contains string-trim))
(export ast-patch-build-request
        ast-patch-validate-request
        ast-patch-from-style-finding
        ast-patch-project-vector
        EnableAstPatchRepair)

(def +ast-patch-op-wire-names+
  '((replace-item       . "replace_item")
    (append-to-block    . "append_to_block")
    (insert-before-node . "insert_before_node")
    (insert-after-node  . "insert_after_node")
    (remove-node        . "remove_node")
    (insert-import      . "insert_import")))

(def (non-empty-string? x)
  (and (string? x) (> (string-length (string-trim x)) 0)))

(def (safe-relative-workspace-path? path)
  (and (non-empty-string? path)
       (not (char=? (string-ref path 0) #\/))
       (not (string-contains path ".."))
       (not (string-contains path "\x0;"))))

(def (op-wire-name op)
  (let ((entry (assq op +ast-patch-op-wire-names+)))
    (and entry (cdr entry))))

(def (spec-ref spec key default)
  (cond
    ((and (list? spec) (assq key spec)) => cdr)
    ((and (object? spec) (.slot? spec key)) (.ref spec key))
    (else default)))

;;; Fail-closed structural validation for an AST patch request object.
(def (ast-patch-validate-request req)
  (and (object? req)
       (.slot? req 'kind)
       (eq? (.ref req 'kind) 'marlin.repair.ast-patch-request)
       (.slot? req 'path)
       (safe-relative-workspace-path? (.ref req 'path))
       (.slot? req 'op)
       (let ((op (.ref req 'op)))
         (and (op-wire-name op)
              (case op
                ((replace-item append-to-block insert-before-node insert-after-node)
                 (and (.slot? req 'target) (non-empty-string? (.ref req 'target))
                      (.slot? req 'snippet) (string? (.ref req 'snippet))))
                ((remove-node)
                 (and (.slot? req 'target) (non-empty-string? (.ref req 'target))))
                ((insert-import)
                 (and (.slot? req 'import-statement)
                      (non-empty-string? (.ref req 'import-statement))))
                (else #f))))
       (let ((preimage (and (.slot? req 'expected-preimage-hash)
                            (.ref req 'expected-preimage-hash))))
         (or (not preimage) (non-empty-string? preimage)))
       (let ((lang (and (.slot? req 'language) (.ref req 'language))))
         (or (not lang) (non-empty-string? lang)))
       #t))

;;; Build and validate a sealed AST patch request POO value from an alist or POO spec:
;;;   `((path . "crates/foo/src/lib.rs") (op . replace-item) (target . "fn#add")
;;;     (snippet . "pub fn add(a: i32, b: i32) -> i32 { a + b }")
;;;     (expected-preimage-hash . "sha256:...") (dry-run? . #t) (auto-calibrate? . #t))`
(def (ast-patch-build-request spec)
  (let* ((path-val (spec-ref spec 'path #f))
         (op-val (spec-ref spec 'op #f))
         (op-wire-val (op-wire-name op-val))
         (target-val (spec-ref spec 'target #f))
         (snippet-val (spec-ref spec 'snippet #f))
         (import-val (spec-ref spec 'import-statement #f))
         (lang-val (spec-ref spec 'language #f))
         (preimage-val (spec-ref spec 'expected-preimage-hash #f))
         (dry-run-val (if (spec-ref spec 'dry-run? #t) #t #f))
         (auto-cal-val (if (spec-ref spec 'auto-calibrate? #t) #t #f))
         (rule-id-val (spec-ref spec 'rule-id #f))
         (req (seal-object!
                (.o kind: 'marlin.repair.ast-patch-request
                    path: path-val
                    op: op-val
                    op-wire: op-wire-val
                    target: target-val
                    snippet: snippet-val
                    import-statement: import-val
                    language: lang-val
                    expected-preimage-hash: preimage-val
                    dry-run?: dry-run-val
                    auto-calibrate?: auto-cal-val
                    rule-id: rule-id-val))))
    (unless (ast-patch-validate-request req)
      (error "invalid AST patch request: requires safe relative path, valid ASP op, and matching target/snippet/import-statement"
             path-val op-val))
    req))

;;; Lower an `ast-style` finding tuple `(rule_id class entity_id path name line)`
;;; + authoritative `locator` + replacement `snippet` (or alist of options) into
;;; a validated dry-run-first AST patch request. Refuses non-patchable classes
;;; (`pre-gen-contract`) fail-closed.
(def (ast-patch-from-style-finding finding locator snippet-or-opts)
  (unless (and (list? finding) (= (length finding) 6))
    (error "style finding must be a 6-tuple (rule_id class entity_id path name line)" finding))
  (let* ((rule-id (list-ref finding 0))
         (class-str (list-ref finding 1))
         (path (list-ref finding 3))
         (opts (if (string? snippet-or-opts)
                 (list (cons 'snippet snippet-or-opts))
                 snippet-or-opts)))
    (unless (or (string=? class-str "propose-ast-patch")
                (string=? class-str "auto-elevate-local"))
      (error "style finding class is not eligible for AST patch proposal" rule-id class-str))
    (ast-patch-build-request
      (list (cons 'path path)
            (cons 'op (spec-ref opts 'op 'replace-item))
            (cons 'target locator)
            (cons 'snippet (spec-ref opts 'snippet #f))
            (cons 'import-statement (spec-ref opts 'import-statement #f))
            (cons 'language (spec-ref opts 'language #f))
            (cons 'expected-preimage-hash (spec-ref opts 'expected-preimage-hash #f))
            (cons 'dry-run? (spec-ref opts 'dry-run? #t))
            (cons 'auto-calibrate? (spec-ref opts 'auto-calibrate? #t))
            (cons 'rule-id rule-id)))))

;;; Project a validated AST patch request to a zero-JSON rooted Scheme 10-vector
;;; matching Rust `AspHttpAstPatchRequest` / `AspHttpAstPatchOp`:
;;;   #(ast-patch-request path language preimage-hash op-wire target snippet import-statement dry-run? auto-calibrate?)
(def (ast-patch-project-vector req)
  (unless (ast-patch-validate-request req)
    (error "cannot project invalid AST patch request"))
  (vector 'ast-patch-request
          (.ref req 'path)
          (.ref req 'language)
          (.ref req 'expected-preimage-hash)
          (.ref req 'op-wire)
          (.ref req 'target)
          (.ref req 'snippet)
          (.ref req 'import-statement)
          (.ref req 'dry-run?)
          (.ref req 'auto-calibrate?)))

;;; Yuanxu feature refiner enabling `+ast-patch` on `Marlin.repair`.
(def (EnableAstPatchRepair inherited)
  (.o (:: self inherited)
      ast-patch: (.o build-request: ast-patch-build-request
                     validate-request: ast-patch-validate-request
                     from-style-finding: ast-patch-from-style-finding
                     project-vector: ast-patch-project-vector)))
