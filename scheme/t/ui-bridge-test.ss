;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; End-to-end test for Marlin's Unified Scheme Bridge Hub (`scheme/ui-bridge`):
;;; verifies modular namespace discovery, real Org Mode (`orgize`) Document IR
;;; projection (with UTF-8 Chinese + code-block isolation), Markdown projection,
;;; and `doc:parse` auto-dispatch.
(import (only-in :marlin-agent-core/ui-bridge/funcs
                 marlin-scheme-bridge-list-namespaces
                 marlin-scheme-bridge-dispatch))

(def (facet-ref rec key)
  (let* ((facets (vector-ref rec 7))
         (n (vector-length facets)))
    (let loop ((i 0))
      (if (>= i n)
        #f
        (let ((kv (vector-ref facets i)))
          (if (string=? (vector-ref kv 0) key)
            (vector-ref kv 1)
            (loop (+ i 1))))))))

;; 1. Verify namespace discovery exposes org, markdown, doc.
(def ns-list (vector->list (marlin-scheme-bridge-list-namespaces #f)))
(unless (and (member "org" ns-list) (member "markdown" ns-list) (member "doc" ns-list))
  (error "expected org, markdown, doc namespaces in Scheme bridge" ns-list))

;; 2. Verify real Org Mode projection via `doc:parse` on .org path (including Chinese UTF-8).
(def org-sample
  "#+title: 统一 Scheme Bridge\n* TODO [#A] 实现 Org 渲染 :org:ui:\nSCHEDULED: <2026-11-28 Sat> DEADLINE: <2026-12-01 Tue>\n:PROPERTIES:\n:ID: org-bridge-1\n:END:\n正文 *加粗* 和 =代码=。\n- [X] 统一 Scheme Bridge\n- [ ] Org Agenda\n| 层级 | 职责 |\n|------+------|\n| Scheme | 解析 |\n#+begin_src rust\n* TODO not a headline\nfn main() {}\n#+end_src\n#+begin_quote\nPure modular bridge.\n#+end_quote\n")

(def org-blocks (marlin-scheme-bridge-dispatch (vector "doc" "parse" "README.org" org-sample)))
(def block-kinds (map (lambda (r) (facet-ref r "block_kind")) (vector->list org-blocks)))
(unless (and (member "heading" block-kinds)
             (member "org-planning" block-kinds)
             (member "drawer" block-kinds)
             (member "paragraph" block-kinds)
             (member "list" block-kinds)
             (member "table" block-kinds)
             (member "code" block-kinds)
             (member "quote" block-kinds))
  (error "missing expected Org block kinds" block-kinds))

;; Verify headline TODO/priority/tags & code-block fake-headline isolation.
(def headings
  (filter (lambda (r) (equal? (facet-ref r "block_kind") "heading"))
          (vector->list org-blocks)))
(unless (= (length headings) 2)
  (error "expected 2 headings (title + 1 real headline, ignoring src-block)" (length headings)))
(def main-h (cadr headings))
(unless (and (equal? (facet-ref main-h "todo_word") "TODO")
             (equal? (facet-ref main-h "priority") "A")
             (equal? (facet-ref main-h "tags") "org,ui"))
  (error "unexpected Org headline facets" main-h))

;; 3. Verify `org:agenda` extracts TODO items cleanly.
(def agenda (marlin-scheme-bridge-dispatch (vector "org" "agenda" "README.org" org-sample)))
(unless (and (= (vector-length agenda) 1)
             (equal? (facet-ref (vector-ref agenda 0) "todo_word") "TODO"))
  (error "expected 1 Org agenda TODO item" agenda))

;; 4. Verify Markdown projection via `doc:parse` on .md path.
(def md-sample
  "# Unified Bridge\n\n> [!NOTE]\n> Markdown & Org share one GPU renderer.\n\n- [x] Done\n- [ ] Next\n\n| Col | Val |\n|-----|-----|\n| A   | 1   |\n\n```go\nfunc main() {}\n```\n")
(def md-blocks (marlin-scheme-bridge-dispatch (vector "doc" "parse" "GUIDE.md" md-sample)))
(def md-kinds (map (lambda (r) (facet-ref r "block_kind")) (vector->list md-blocks)))
(unless (and (member "heading" md-kinds)
             (member "quote" md-kinds)
             (member "list" md-kinds)
             (member "table" md-kinds)
             (member "code" md-kinds))
  (error "missing expected Markdown block kinds" md-kinds))

(displayln "UI-BRIDGE-OK")
