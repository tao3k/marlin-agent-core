;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure functional Modern Helm state machine, two-stage Consult/Orderless query
;;; parser, virtual-viewport TUI renderer, Embark action dispatcher, and native
;;; `scheme/search-engine` RootedSchemeObject Playbook compiler.
(import (only-in :clan/poo/object .get .o seal-object!)
        (only-in :std/string/misc string-contains string-trim)
        (only-in "../ui-elements/funcs.ss" ui-rows ui-text)
        (only-in "../search-engine/funcs.ss"
                 intersect-plan
                 make-search-plan
                 project-search-plan
                 rg-leaf
                 tantivy-leaf
                 topology-leaf)
        "./objects.ss")
(export make-helm-source
        make-helm-candidate
        make-helm-action
        default-helm-sources
        default-helm-actions
        helm-open-session
        helm-parse-input
        helm-compile-search-plan
        helm-compile-plan-vector
        helm-filter-candidates
        helm-update-input
        helm-apply-results
        helm-move-selection
        helm-toggle-mark
        helm-next-source
        helm-toggle-preview
        helm-toggle-actions
        helm-dispatch-action
        helm-cancel
        helm-visible-candidates
        helm-selected-candidate
        helm-render-lines
        helm-project-ui-elements)

(def (make-helm-source id label kind: (kind 'search) priority: (priority 10) async?: (async? #t))
  (seal-object!
    (.o (:: @ HelmSource.)
        (id := id)
        (label := label)
        (kind := kind)
        (priority := priority)
        (async? := async?))))

(def (make-helm-candidate
       id
       display
       source-id: (source-id 'search)
       path: (path #f)
       line: (line #f)
       language: (language #f)
       axes: (axes '())
       score: (score 0)
       annotation: (annotation "")
       preview-lines: (preview-lines '()))
  (seal-object!
    (.o (:: @ HelmCandidate.)
        (id := id)
        (source-id := source-id)
        (display := display)
        (path := path)
        (line := line)
        (language := language)
        (axes := axes)
        (score := score)
        (annotation := annotation)
        (preview-lines := preview-lines))))

(def (make-helm-action id key label scope: (scope 'single))
  (seal-object!
    (.o (:: @ HelmAction.)
        (id := id)
        (key := key)
        (label := label)
        (scope := scope))))

(def (default-helm-sources)
  (list
    (make-helm-source 'search "Workspace Search" kind: 'search priority: 10 async?: #t)
    (make-helm-source 'buffers "Buffers" kind: 'buffers priority: 20 async?: #f)
    (make-helm-source 'commands "Commands (M-x)" kind: 'commands priority: 30 async?: #f)
    (make-helm-source 'org "Org Context" kind: 'org priority: 40 async?: #t)))

(def (default-helm-actions)
  (list
    (make-helm-action 'open "RET" "Open / Jump" scope: 'single)
    (make-helm-action 'preview "C-o" "Toggle Preview" scope: 'single)
    (make-helm-action 'mark "TAB" "Toggle Mark" scope: 'single)
    (make-helm-action 'export-context "C-e" "Export to Shared Org Context" scope: 'marked)
    (make-helm-action 'dispatch-agent "C-a" "Dispatch Hits to Agent" scope: 'marked)))

(def (split-whitespace text)
  (let loop ((chars (string->list text)) (cur '()) (acc '()))
    (cond
      ((null? chars)
       (reverse (if (null? cur) acc (cons (list->string (reverse cur)) acc))))
      ((char-whitespace? (car chars))
       (loop (cdr chars)
             '()
             (if (null? cur) acc (cons (list->string (reverse cur)) acc))))
      (else
       (loop (cdr chars) (cons (car chars) cur) acc)))))

(def (split-two-stage text)
  (let ((idx (string-index text #\#)))
    (if idx
      (values (string-trim (substring text 0 idx))
              (string-trim (substring text (+ idx 1) (string-length text))))
      (values (string-trim text) ""))))

(def (append-unique items value)
  (if (member value items) items (append items (list value))))

(def (helm-parse-input raw-text)
  (let-values (((async-raw local-raw) (split-two-stage raw-text)))
    (let loop ((tokens (split-whitespace async-raw))
               (pattern-parts '())
               (languages '())
               (documents '())
               (path-prefix #f)
               (extension #f))
      (if (null? tokens)
        (seal-object!
          (.o (:: @ HelmParsedInput.)
              (raw-text := raw-text)
              (async-pattern := (string-join (reverse pattern-parts) " "))
              (local-tokens := (split-whitespace local-raw))
              (languages := languages)
              (documents := documents)
              (path-prefix := path-prefix)
              (extension := extension)))
        (let ((tok (car tokens)))
          (cond
            ((string-prefix? "@lang:" tok)
             (let ((v (substring tok 6 (string-length tok))))
               (loop (cdr tokens) pattern-parts
                     (if (string=? v "") languages (append-unique languages v))
                     documents path-prefix extension)))
            ((string-prefix? "@doc:" tok)
             (let ((v (substring tok 5 (string-length tok))))
               (loop (cdr tokens) pattern-parts languages
                     (if (string=? v "") documents (append-unique documents v))
                     path-prefix extension)))
            ((string-prefix? "@prefix:" tok)
             (let ((v (substring tok 8 (string-length tok))))
               (loop (cdr tokens) pattern-parts languages documents
                     (if (string=? v "") path-prefix v) extension)))
            ((string-prefix? "@ext:" tok)
             (let ((v (substring tok 5 (string-length tok))))
               (loop (cdr tokens) pattern-parts languages documents
                     path-prefix (if (string=? v "") extension v))))
            (else
             (loop (cdr tokens) (cons tok pattern-parts)
                   languages documents path-prefix extension))))))))

(def (helm-compile-search-plan parsed workspace: (workspace "marlin-main"))
  (let* ((pattern (.get parsed async-pattern))
         (languages (.get parsed languages))
         (documents (.get parsed documents))
         (path-prefix (.get parsed path-prefix))
         (extension (.get parsed extension))
         (effective-pattern (if (string=? pattern "") "marlin" pattern))
         (rg (rg-leaf "-n" effective-pattern))
         (tantivy (tantivy-leaf
                    (string-append "title:\"" effective-pattern "\"^2 OR body:" effective-pattern)))
         (topo (if (or path-prefix extension)
                 (topology-leaf member-kind: "file"
                                path-prefix: path-prefix
                                extension: extension)
                 #f))
         (composition (if topo
                        (intersect-plan rg tantivy topo)
                        (intersect-plan rg tantivy))))
    (make-search-plan composition
                      workspace: workspace
                      languages: (if (and (null? languages) (null? documents))
                                   '(rust)
                                   (map string->symbol languages))
                      documents: (map string->symbol documents))))

(def (helm-compile-plan-vector raw-text workspace: (workspace "marlin-main"))
  (project-search-plan
    (helm-compile-search-plan (helm-parse-input raw-text) workspace: workspace)))

(def (candidate-searchable-haystack candidate)
  (string-join
    (filter string?
            (list (.get candidate display)
                  (.get candidate path)
                  (.get candidate annotation)
                  (symbol->string (.get candidate source-id))))
    " "))

(def (string-contains-ci haystack needle)
  (string-contains (string-downcase haystack) (string-downcase needle)))

(def (token-matches-haystack? haystack token)
  (cond
    ((string=? token "") #t)
    ((and (> (string-length token) 1) (char=? (string-ref token 0) #\!))
     (not (string-contains-ci haystack (substring token 1 (string-length token)))))
    (else
     (and (string-contains-ci haystack token) #t))))

(def (all-tokens-match? haystack tokens)
  (let loop ((rest tokens))
    (cond
      ((null? rest) #t)
      ((token-matches-haystack? haystack (car rest)) (loop (cdr rest)))
      (else #f))))

(def (helm-filter-candidates candidates active-source local-tokens)
  (filter
    (lambda (cand)
      (and (or (eq? active-source 'all)
               (eq? (.get cand source-id) active-source))
           (all-tokens-match? (candidate-searchable-haystack cand) local-tokens)))
    candidates))

(def (clamp-index idx len)
  (cond
    ((<= len 0) 0)
    ((< idx 0) 0)
    ((>= idx len) (- len 1))
    (else idx)))

(def (recompute-viewport-offset selected-index current-offset height total)
  (cond
    ((<= total 0) 0)
    ((< selected-index current-offset) selected-index)
    ((>= selected-index (+ current-offset height))
     (+ (- selected-index height) 1))
    (else
     (min current-offset (max 0 (- total height))))))

(def (prune-marks marked-ids candidates)
  (let ((valid-ids (map (lambda (c) (.get c id)) candidates)))
    (filter (lambda (mid) (member mid valid-ids)) marked-ids)))

(def (helm-open-session
       workspace: (workspace "marlin-main")
       sources: (sources (default-helm-sources))
       initial-candidates: (initial-candidates '())
       viewport-height: (viewport-height 8)
       layout: (layout 'split-preview))
  (let ((parsed (helm-parse-input "")))
    (seal-object!
      (.o (:: @ HelmSession.)
          (open? := #t)
          (workspace := workspace)
          (input-text := "")
          (parsed := parsed)
          (epoch := 1)
          (sources := sources)
          (active-source := 'all)
          (raw-candidates := initial-candidates)
          (candidates := initial-candidates)
          (selected-index := 0)
          (marked-ids := '())
          (viewport-offset := 0)
          (viewport-height := (max 1 viewport-height))
          (layout := layout)
          (action-menu-open? := #f)
          (generation-digest := #f)
          (last-action-receipt := #f)))))

(def (rebuild-session-view session next-raw next-active-source next-parsed next-epoch next-gen)
  (let* ((filtered (helm-filter-candidates next-raw next-active-source (.get next-parsed local-tokens)))
         (total (length filtered))
         (sel (clamp-index (.get session selected-index) total))
         (vh (.get session viewport-height))
         (voff (recompute-viewport-offset sel (.get session viewport-offset) vh total))
         (marks (prune-marks (.get session marked-ids) filtered)))
    (seal-object!
      (.o (:: @ session)
          (parsed := next-parsed)
          (epoch := next-epoch)
          (active-source := next-active-source)
          (raw-candidates := next-raw)
          (candidates := filtered)
          (selected-index := sel)
          (marked-ids := marks)
          (viewport-offset := voff)
          (generation-digest := next-gen)))))

(def (helm-update-input session next-text)
  (let* ((prev-parsed (.get session parsed))
         (next-parsed (helm-parse-input next-text))
         (async-changed?
           (not (and (string=? (.get prev-parsed async-pattern) (.get next-parsed async-pattern))
                     (equal? (.get prev-parsed languages) (.get next-parsed languages))
                     (equal? (.get prev-parsed documents) (.get next-parsed documents))
                     (equal? (.get prev-parsed path-prefix) (.get next-parsed path-prefix))
                     (equal? (.get prev-parsed extension) (.get next-parsed extension)))))
         (next-epoch (if async-changed? (+ (.get session epoch) 1) (.get session epoch)))
         (next-session (rebuild-session-view
                         session
                         (.get session raw-candidates)
                         (.get session active-source)
                         next-parsed
                         next-epoch
                         (.get session generation-digest))))
    (seal-object!
      (.o (:: @ next-session)
          (input-text := next-text)
          (action-menu-open? := #f)))))

;;; Epoch-fenced async result application: responses whose epoch does not match
;;; the active `session.epoch` (stale or out-of-band) are dropped fail-closed.
(def (helm-apply-results session result-epoch generation-digest new-candidates)
  (if (or (not (.get session open?))
          (not (= result-epoch (.get session epoch))))
    session
    (rebuild-session-view
      session
      new-candidates
      (.get session active-source)
      (.get session parsed)
      result-epoch
      generation-digest)))

(def (helm-move-selection session delta)
  (let* ((total (length (.get session candidates)))
         (next-idx (clamp-index (+ (.get session selected-index) delta) total))
         (next-off (recompute-viewport-offset
                     next-idx (.get session viewport-offset)
                     (.get session viewport-height) total)))
    (seal-object!
      (.o (:: @ session)
          (selected-index := next-idx)
          (viewport-offset := next-off)))))

(def (helm-selected-candidate session)
  (let ((cands (.get session candidates))
        (idx (.get session selected-index)))
    (if (and (>= idx 0) (< idx (length cands)))
      (list-ref cands idx)
      #f)))

(def (helm-toggle-mark session)
  (let ((selected (helm-selected-candidate session)))
    (if (not selected)
      session
      (let* ((cid (.get selected id))
             (marks (.get session marked-ids))
             (next-marks (if (member cid marks)
                           (filter (lambda (m) (not (equal? m cid))) marks)
                           (append marks (list cid)))))
        (seal-object!
          (.o (:: @ session)
              (marked-ids := next-marks)))))))

(def (helm-next-source session)
  (let* ((source-ids (cons 'all (map (lambda (s) (.get s id)) (.get session sources))))
         (cur (.get session active-source))
         (tail (memq cur source-ids))
         (next-id (if (and tail (pair? (cdr tail))) (cadr tail) (car source-ids))))
    (rebuild-session-view
      session
      (.get session raw-candidates)
      next-id
      (.get session parsed)
      (.get session epoch)
      (.get session generation-digest))))

(def (helm-toggle-preview session)
  (let ((next-layout (if (eq? (.get session layout) 'split-preview)
                       'bottom-list
                       'split-preview)))
    (seal-object!
      (.o (:: @ session)
          (layout := next-layout)))))

(def (helm-toggle-actions session)
  (seal-object!
    (.o (:: @ session)
        (action-menu-open? := (not (.get session action-menu-open?))))))

(def (helm-dispatch-action session action-id)
  (let* ((selected (helm-selected-candidate session))
         (marks (.get session marked-ids))
         (targets (cond
                    ((not (null? marks)) marks)
                    (selected (list (.get selected id)))
                    (else '())))
         (receipt (seal-object!
                    (.o (:: @ HelmActionReceipt.)
                        (action-id := action-id)
                        (epoch := (.get session epoch))
                        (generation-digest := (.get session generation-digest))
                        (target-ids := targets)
                        (status := (if (null? targets) 'empty 'accepted))))))
    (seal-object!
      (.o (:: @ session)
          (action-menu-open? := #f)
          (last-action-receipt := receipt)))))

(def (helm-cancel session)
  (seal-object!
    (.o (:: @ session)
        (open? := #f)
        (action-menu-open? := #f))))

(def (take-slice items offset count)
  (let loop ((rest items) (i 0) (acc '()))
    (cond
      ((or (null? rest) (>= (length acc) count)) (reverse acc))
      ((< i offset) (loop (cdr rest) (+ i 1) acc))
      (else (loop (cdr rest) (+ i 1) (cons (car rest) acc))))))

(def (helm-visible-candidates session)
  (take-slice (.get session candidates)
              (.get session viewport-offset)
              (.get session viewport-height)))

(def (short-digest digest)
  (if (and (string? digest) (> (string-length digest) 18))
    (substring digest 0 18)
    (or digest "pending")))

(def (render-candidate-row cand idx selected-index marked-ids)
  (let* ((cursor (if (= idx selected-index) ">" " "))
         (mark (if (member (.get cand id) marked-ids) "*" " "))
         (ann (.get cand annotation))
         (ann-part (if (string=? ann "") "" (string-append "  [" ann "]"))))
    (string-append cursor mark " " (.get cand display) ann-part)))

;;; Render a bounded O(viewport-height) TUI frame for Ratatui / scheme/t tests.
(def (helm-render-lines session)
  (let* ((total (length (.get session candidates)))
         (sel (.get session selected-index))
         (voff (.get session viewport-offset))
         (marks (.get session marked-ids))
         (visible (helm-visible-candidates session))
         (header (string-append
                   "HELM [" (symbol->string (.get session active-source))
                   "] epoch=" (number->string (.get session epoch))
                   " hits=" (number->string total)
                   " marked=" (number->string (length marks))
                   " gen=" (short-digest (.get session generation-digest))))
         (prompt (string-append "Helm> " (.get session input-text)))
         (rows (let loop ((rest visible) (i voff) (acc '()))
                 (if (null? rest)
                   (reverse acc)
                   (loop (cdr rest)
                         (+ i 1)
                         (cons (render-candidate-row (car rest) i sel marks) acc)))))
         (selected (helm-selected-candidate session))
         (preview (if (and (eq? (.get session layout) 'split-preview)
                           selected
                           (not (null? (.get selected preview-lines))))
                    (cons (string-append "--- Preview: " (.get selected display) " ---")
                          (.get selected preview-lines))
                    '()))
         (actions (if (.get session action-menu-open?)
                    (list "--- Actions: [RET] Open  [C-o] Preview  [TAB] Mark  [C-e] Export-Context  [C-a] Dispatch-Agent ---")
                    '())))
    (append (list header prompt)
            (if (null? rows) (list "  (no matches)") rows)
            preview
            actions)))

;;; Project a `HelmSession.` directly into declarative Marlin `ui-elements`
;;; (`ui-rows` + `ui-text`) for Ratatui rendering and `UiPackage.` buffers.
(def (helm-project-ui-elements session)
  (let* ((total (length (.get session candidates)))
         (sel (.get session selected-index))
         (voff (.get session viewport-offset))
         (marks (.get session marked-ids))
         (visible (helm-visible-candidates session))
         (header (string-append
                   "HELM [" (symbol->string (.get session active-source))
                   "] epoch=" (number->string (.get session epoch))
                   " hits=" (number->string total)
                   " marked=" (number->string (length marks))
                   " gen=" (short-digest (.get session generation-digest))))
         (prompt (string-append "Helm> " (.get session input-text)))
         (row-elements
           (if (null? visible)
             (list (ui-text (list (cons 'muted "  (no matches)"))))
             (let loop ((rest visible) (i voff) (acc '()))
               (if (null? rest)
                 (reverse acc)
                 (let* ((cand (car rest))
                        (selected? (= i sel))
                        (marked? (and (member (.get cand id) marks) #t))
                        (prefix (string-append (if selected? "> " "  ")
                                               (if marked? "* " "  ")))
                        (ann (.get cand annotation))
                        (spans (append
                                 (list (cons (if selected? 'accent 'muted) prefix)
                                       (cons (if selected? 'info 'normal) (.get cand display)))
                                 (if (string=? ann "")
                                   '()
                                   (list (cons 'muted (string-append "  [" ann "]")))))))
                   (loop (cdr rest) (+ i 1) (cons (ui-text spans) acc)))))))
         (selected (helm-selected-candidate session))
         (preview-elements
           (if (and (eq? (.get session layout) 'split-preview)
                    selected
                    (not (null? (.get selected preview-lines))))
             (cons (ui-text (list (cons 'info (string-append "--- Preview: " (.get selected display) " ---"))))
                   (map (lambda (line) (ui-text (list (cons 'normal line))))
                        (.get selected preview-lines)))
             '()))
         (action-elements
           (if (.get session action-menu-open?)
             (list (ui-text (list (cons 'warning "--- Actions: [RET] Open  [C-o] Preview  [TAB] Mark  [C-e] Export-Context  [C-a] Dispatch-Agent ---"))))
             '())))
    (ui-rows
      (append
        (list (ui-text (list (cons 'accent header)))
              (ui-text (list (cons 'info prompt))))
        row-elements
        preview-elements
        action-elements))))
