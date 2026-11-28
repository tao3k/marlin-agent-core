;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure functional Magit section tree, Human-AI audit progress state,
;;; risk-priority sorting, and Aitia-grounded AI prompt builders.
(import (only-in :clan/poo/object .cc .get .o)
        (only-in :clan/poo/type validate)
        (only-in :std/list/list filter)
        (only-in "../ui-meow/key-bindings.ss" UiMeowScope)
        "./objects.ss"
        "./types.ss")
(export marlin-magit-make-state
        marlin-magit-toggle-fold
        marlin-magit-set-visibility-level
        marlin-magit-step-cursor
        marlin-magit-mark-hunk-audit
        marlin-magit-audit-progress
        marlin-magit-sort-hunks-by-risk
        marlin-magit-fuse-evidence-risk
        marlin-magit-build-ai-audit-prompt
        marlin-magit-build-ai-commit-prompt
        marlin-magit-transient-for
        marlin-ui-magit-project-grpc-surface)

(def (risk-weight risk)
  (let ((pair (assq risk MagitRiskOrder)))
    (if pair (cdr pair) 0)))

(def (assoc-set alist key val)
  (cons (cons key val)
        (filter (lambda (p) (not (equal? (car p) key))) alist)))

;; Create initial pure Magit buffer state:
;;   visibility-level: 1 (sections), 2 (files), 3 (hunks), 4 (full diff lines)
;;   folded-ids: list of collapsed section/file/hunk string IDs
;;   audit-map: alist of (hunk-id . 'unreviewed|'audited|'flagged)
(def (marlin-magit-make-state)
  (.o cursor: 0
      visibility-level: 3
      folded-ids: '()
      audit-map: '()
      active-transient: #f))

;; Toggle folding for a section, file, or hunk ID (Magit TAB).
(def (marlin-magit-toggle-fold state section-id)
  (let* ((folded (.get state folded-ids))
         (next (if (member section-id folded)
                 (filter (lambda (x) (not (equal? x section-id))) folded)
                 (cons section-id folded))))
    (.cc state folded-ids: next)))

;; Set Magit section visibility level 1..4 (keys 1/2/3/4) and clear ad-hoc folds.
(def (marlin-magit-set-visibility-level state level)
  (let ((clamped (max 1 (min 4 level))))
    (.cc state visibility-level: clamped folded-ids: '())))

;; Move cursor up/down across selectable Magit rows (keys j/k).
(def (marlin-magit-step-cursor state delta total-rows)
  (if (<= total-rows 0)
    (.cc state cursor: 0)
    (let* ((cur (.get state cursor))
           (next (max 0 (min (- total-rows 1) (+ cur delta)))))
      (.cc state cursor: next))))

;; Mark a hunk as 'unreviewed, 'audited, or 'flagged (Human-AI review checklist).
(def (marlin-magit-mark-hunk-audit state hunk-id audit-status)
  (validate MagitAuditState audit-status)
  (.cc state audit-map: (assoc-set (.get state audit-map) hunk-id audit-status)))

;; Compute Human-AI audit progress across a list of MagitHunkContract objects:
;; returns POO record { audited, flagged, unreviewed, total, high-risk-remaining }.
(def (marlin-magit-audit-progress state hunks)
  (let ((amap (.get state audit-map)))
    (let loop ((rest hunks) (aud 0) (flg 0) (unrev 0) (high-rem 0))
      (if (null? rest)
        (.o audited: aud
            flagged: flg
            unreviewed: unrev
            total: (+ aud flg unrev)
            high-risk-remaining: high-rem)
        (let* ((h (car rest))
               (hid (.get h hunk-id))
               (entry (assoc hid amap))
               (st (if entry (cdr entry) 'unreviewed))
               (is-high (eq? (.get h risk) 'high)))
          (cond
            ((eq? st 'audited)
             (loop (cdr rest) (+ aud 1) flg unrev high-rem))
            ((eq? st 'flagged)
             (loop (cdr rest) aud (+ flg 1) unrev (if is-high (+ high-rem 1) high-rem)))
            (else
             (loop (cdr rest) aud flg (+ unrev 1) (if is-high (+ high-rem 1) high-rem)))))))))

(def (insert-sorted elem sorted less?)
  (cond
    ((null? sorted) (list elem))
    ((less? elem (car sorted)) (cons elem sorted))
    (else (cons (car sorted) (insert-sorted elem (cdr sorted) less?)))))

(def (sort-list items less?)
  (let loop ((rest items) (acc '()))
    (if (null? rest)
      acc
      (loop (cdr rest) (insert-sorted (car rest) acc less?)))))

;; Priority-order hunks for cognitive-load reduction: HIGH -> MEDIUM -> LOW -> INFO,
;; breaking ties deterministically by file-path then hunk-id.
(def (marlin-magit-sort-hunks-by-risk hunks)
  (sort-list hunks
             (lambda (a b)
               (let ((wa (risk-weight (.get a risk)))
                     (wb (risk-weight (.get b risk))))
                 (if (= wa wb)
                   (string<? (string-append (.get a file-path) ":" (.get a hunk-id))
                             (string-append (.get b file-path) ":" (.get b hunk-id)))
                   (> wa wb))))))

;; Honest Multi-Source Evidence Chain risk fusion:
;; Never fabricates HIGH risk from blind keyword matching!
;; Inputs:
;;   aitia-status: 'online-admit | 'online-findings | 'offline
;;   aitia-finding-count: exact integer of real Lambda Aitia g2Findings hitting this hunk
;;   formal-status: 'verified | 'model-present-unrun | 'none
;;   joint-verdict: 'unreviewed | 'audited | 'flagged | 'disputed
;;   has-conflict?: boolean
(def (marlin-magit-fuse-evidence-risk aitia-status aitia-finding-count formal-status joint-verdict has-conflict?)
  (cond
    (has-conflict?
     (.o risk: 'high
         reason: "Unresolved merge/jj conflict requires human + AI 3-way resolution"))
    ((or (eq? joint-verdict 'flagged) (eq? joint-verdict 'disputed))
     (.o risk: 'high
         reason: "Flagged/disputed by Human-AI joint audit evidence"))
    ((> aitia-finding-count 0)
     (.o risk: 'high
         reason: (string-append "Lambda Aitia G2 defense audit reported "
                                (number->string aitia-finding-count)
                                " finding(s) on this hunk")))
    ((eq? aitia-status 'offline)
     (.o risk: 'unassessed
         reason: "Lambda Aitia sidecar offline — structural diffstat only (no risk verdict asserted)"))
    ((and (eq? joint-verdict 'audited) (eq? formal-status 'verified))
     (.o risk: 'info
         reason: "Verified by Lambda Aitia (ADMIT), Formal Evidence, and Human-AI joint audit"))
    ((eq? joint-verdict 'audited)
     (.o risk: 'low
         reason: "Lambda Aitia ADMIT (0 findings) + Human-AI audited"))
    ((eq? formal-status 'model-present-unrun)
     (.o risk: 'medium
         reason: "Lambda Aitia ADMIT (0 findings) · Formal model exists (verification pending)"))
    (else
     (.o risk: 'low
         reason: "Lambda Aitia ADMIT (0 findings) · awaiting human/LLM confirmation"))))

;; Build a structured, Aitia-grounded Human-AI Code Audit prompt for one hunk.
(def (marlin-magit-build-ai-audit-prompt aitia hunk)
  (validate MagitAitiaContextContract aitia)
  (validate MagitHunkContract hunk)
  (let* ((ev (.get aitia evidence-id))
         (br (.get aitia branch))
         (cid (.get aitia change-id))
         (sha (.get aitia commit-sha))
         (ref-str (cond
                    ((and (string? cid) (not (string=? cid "")))
                     (string-append "jj@" cid (if (string? sha) (string-append " / git:" sha) "")))
                    ((string? sha) (string-append "git:" sha))
                    (else "unborn"))))
    (string-append
      "[Magit AI Code Audit | Aitia: " ev "]\n"
      "Repository branch: " br " (" ref-str ")\n"
      "File: " (.get hunk file-path) " (" (.get hunk header) ")\n"
      "Triage: risk=" (symbol->string (.get hunk risk))
      " concern=" (symbol->string (.get hunk concern))
      " — " (.get hunk risk-reason) "\n\n"
      "Diff hunk:\n```diff\n"
      (if (string? (.get hunk preview)) (.get hunk preview) "")
      "\n```\n\n"
      "Audit concisely for a human reviewer:\n"
      "1. Intent & behavioral delta (1-2 sentences)\n"
      "2. Security / invariant / edge-case risks\n"
      "3. Concrete verification or fix suggestion (if needed)")))

;; Build an Aitia-grounded Conventional Commit / jj describe prompt from hunks.
(def (marlin-magit-build-ai-commit-prompt aitia hunks)
  (validate MagitAitiaContextContract aitia)
  (let* ((sorted (marlin-magit-sort-hunks-by-risk hunks))
         (lines (map (lambda (h)
                       (string-append "- " (.get h file-path)
                                      " [" (symbol->string (.get h risk)) "/"
                                      (symbol->string (.get h concern)) "] "
                                      (.get h risk-reason)))
                     sorted))
         (summary-block (apply string-append (map (lambda (l) (string-append l "\n")) lines))))
    (string-append
      "[Magit AI Commit Draft | Aitia: " (.get aitia evidence-id) "]\n"
      "Branch: " (.get aitia branch) " | Backend: " (symbol->string (.get aitia backend)) "\n"
      "Changed hunks (risk-ordered):\n"
      summary-block
      "\nDraft a concise Conventional Commit message (subject <= 72 chars + optional 2-4 bullet body) and include an `Aitia-Evidence: " (.get aitia evidence-id) "` trailer.")))

;; Look up an authentic Magit transient definition by prefix (e.g. "magit:c").
(def (marlin-magit-transient-for prefix)
  (let loop ((rest MagitTransients))
    (cond
      ((null? rest) #f)
      ((string=? (.get (car rest) prefix) prefix) (car rest))
      (else (loop (cdr rest))))))

(def (key-token->string tok)
  (cond
    ((symbol? tok) (symbol->string tok))
    ((char? tok) (string tok))
    ((string? tok) tok)
    (else "")))

(def (split-spaces str)
  (let ((len (string-length str)))
    (let loop ((i 0) (start 0) (acc '()))
      (cond
        ((>= i len)
         (reverse (if (< start i) (cons (substring str start i) acc) acc)))
        ((char=? (string-ref str i) #\space)
         (loop (+ i 1) (+ i 1)
               (if (< start i) (cons (substring str start i) acc) acc)))
        (else (loop (+ i 1) start acc))))))

(def (strip-action-prefix act)
  (let ((len (string-length act)))
    (if (and (>= len 4) (string=? (substring act 0 4) "cmd:"))
      (substring act 4 len)
      act)))

;; Native C-ABI FFI projection for Rust `marlin-gerbil-scheme` -> `marlin.ui.v1` gRPC:
;; returns pure Scheme vector `#(commands-vec keymap-vec transients-vec)` directly from
;; `UiMeowScope` (`scheme/ui-meow/key-bindings.ss`) and `DoomGitLeaderBindings` /
;; `MagitTransients` (`scheme/ui-magit/objects.ss`) — zero text serialization, zero duplication.
(def (marlin-ui-magit-project-grpc-surface _input)
  (let* ((meow-bindings
           (filter
             (lambda (entry)
               (and (pair? entry)
                    (>= (length entry) 3)
                    (memq (car entry) '(global normal))
                    (symbol? (caddr entry))))
             (.get UiMeowScope bindings)))
         (meow-keymap
           (map (lambda (entry)
                  (vector (symbol->string (car entry))
                          (list->vector (map key-token->string (cadr entry)))
                          (symbol->string (caddr entry))))
                meow-bindings))
         (magit-leader-keymap
           (map (lambda (b)
                  (let ((seq (append (split-spaces (.get b prefix)) (list (.get b key)))))
                    (vector "normal"
                            (list->vector seq)
                            (strip-action-prefix (.get b action)))))
                DoomGitLeaderBindings))
         (leader-cmds
           (filter
             (lambda (x) x)
             (map (lambda (b)
                    (let ((act (.get b action)))
                      (and (>= (string-length act) 4)
                           (string=? (substring act 0 4) "cmd:")
                           (vector (strip-action-prefix act)
                                   (string-append (.get b label) " (" (.get b prefix) " " (.get b key) " / " (.get b emacs-key) ")")
                                   "ui.magit"))))
                  DoomGitLeaderBindings)))
         (transient-cmds
           (let loop ((trs MagitTransients) (acc '()))
             (if (null? trs)
               (reverse acc)
               (let* ((tr (car trs))
                      (heads (.get tr heads))
                      (cmds (filter
                              (lambda (x) x)
                              (map (lambda (h)
                                     (let ((act (.get h action)))
                                       (and (>= (string-length act) 4)
                                            (string=? (substring act 0 4) "cmd:")
                                            (vector (strip-action-prefix act)
                                                    (string-append (.get h label) " (" (.get tr prefix) " " (.get h key) ")")
                                                    "ui.magit"))))
                                   heads))))
                 (loop (cdr trs) (append (reverse cmds) acc))))))
         (dedup-cmds
           (let loop ((rest (append leader-cmds transient-cmds)) (seen '()) (acc '()))
             (if (null? rest)
               (reverse acc)
               (let* ((c (car rest))
                      (name (vector-ref c 0)))
                 (if (member name seen)
                   (loop (cdr rest) seen acc)
                   (loop (cdr rest) (cons name seen) (cons c acc)))))))
         (transients-vec
           (list->vector
             (map (lambda (tr)
                    (vector (.get tr prefix)
                            (.get tr title)
                            (list->vector
                              (append
                                (map (lambda (h)
                                       (vector (.get h key)
                                               (strip-action-prefix (.get h action))
                                               (.get h label)
                                               (.get h exit)))
                                     (.get tr heads))
                                (list (vector "q" "exit" "Quit" #t))))))
                  MagitTransients))))
    (vector (list->vector dedup-cmds)
            (list->vector (append meow-keymap magit-leader-keymap))
            transients-vec)))
