;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import :std/test
        (only-in :std/list/list find)
        (only-in :clan/poo/object .get .o)
        (only-in :clan/poo/mop .new/checked)
        (only-in "../ui-magit/config.ss" UiMagitScope)
        (only-in "../ui-magit/funcs.ss" marlin-ui-magit-project-grpc-surface)
        (only-in "../ui-magit/types.ss" UiMagitScopeContract)
        (only-in "../ui-meow/objects.ss" make-marlin-ui-keymap)
        (only-in "../ui-meow/key-bindings.ss" UiMeowScope))
(export ui-magit-test)

(def (string-contains haystack needle)
  (let ((hlen (string-length haystack))
        (nlen (string-length needle)))
    (cond
      ((= nlen 0) 0)
      ((> nlen hlen) #f)
      (else
       (let loop ((i 0))
         (cond
           ((> (+ i nlen) hlen) #f)
           ((string=? (substring haystack i (+ i nlen)) needle) i)
           (else (loop (+ i 1)))))))))

(def sample-aitia
  (.o evidence-id: "aitia:magit:marlin:kntqzsqt:r1"
      backend: 'jj-colocated
      branch: "main"
      change-id: "kntqzsqt"
      commit-sha: "a1b2c3d4"
      op-id: "op998877"))

(def sample-hunk-low
  (.o hunk-id: "unstaged:README.md:h1:L1"
      file-path: "README.md"
      header: "@@ -1,2 +1,3 @@"
      risk: 'info
      concern: 'tests-docs
      risk-reason: "Tests/docs update (+1/-0)"
      added: 1
      removed: 0
      preview: "+Updated docs"))

(def sample-hunk-high
  (.o hunk-id: "unstaged:src/auth.rs:h1:L10"
      file-path: "src/auth.rs"
      header: "@@ -10,3 +10,5 @@"
      risk: 'high
      concern: 'security-boundary
      risk-reason: "Touches security/boundary surface (bearer, +2/-0)"
      added: 2
      removed: 0
      preview: "+require_bearer(req)?;"))

(def ui-magit-test
  (test-suite "Marlin AI-Native Magit Scheme Policy"
    (test-case "authentic Doom SPC g leader and Magit transients exist"
      (let* ((leaders (.get UiMagitScope doom-leader-bindings))
             (has-gg (find (lambda (b)
                             (and (string=? (.get b prefix) "SPC g")
                                  (string=? (.get b key) "g")))
                           leaders))
             (commit-tr ((.get UiMagitScope transient-for) "magit:c"))
             (diff-tr ((.get UiMagitScope transient-for) "magit:d"))
             (undo-tr ((.get UiMagitScope transient-for) "magit:z")))
        (check has-gg ? (lambda (x) (and x #t)))
        (check commit-tr ? (lambda (x) (and x #t)))
        (check diff-tr ? (lambda (x) (and x #t)))
        (check undo-tr ? (lambda (x) (and x #t)))))

    (test-case "risk-priority sorting puts high-risk hunks first and tracks human audit progress"
      (let* ((sort-fn (.get UiMagitScope sort-hunks-by-risk))
             (sorted (sort-fn (list sample-hunk-low sample-hunk-high)))
             (st0 ((.get UiMagitScope make-state)))
             (prog0 ((.get UiMagitScope audit-progress) st0 sorted))
             (st1 ((.get UiMagitScope mark-hunk-audit) st0 "unstaged:src/auth.rs:h1:L10" 'audited))
             (prog1 ((.get UiMagitScope audit-progress) st1 sorted))
             (st2 ((.get UiMagitScope toggle-fold) st1 "section:unstaged")))
        (check (.get (car sorted) file-path) => "src/auth.rs")
        (check (.get prog0 total) => 2)
        (check (.get prog0 high-risk-remaining) => 1)
        (check (.get prog1 audited) => 1)
        (check (.get prog1 high-risk-remaining) => 0)
        (check (member "section:unstaged" (.get st2 folded-ids)) ? (lambda (x) (and x #t)))))

    (test-case "Aitia-grounded AI audit and commit prompts include evidence ID and diff context"
      (let* ((audit-prompt ((.get UiMagitScope build-ai-audit-prompt) sample-aitia sample-hunk-high))
             (commit-prompt ((.get UiMagitScope build-ai-commit-prompt)
                             sample-aitia (list sample-hunk-low sample-hunk-high))))
        (check (string-contains audit-prompt "aitia:magit:marlin:kntqzsqt:r1") ? (lambda (x) (and x #t)))
        (check (string-contains audit-prompt "src/auth.rs") ? (lambda (x) (and x #t)))
        (check (string-contains audit-prompt "jj@kntqzsqt") ? (lambda (x) (and x #t)))
        (check (string-contains commit-prompt "Aitia-Evidence: aitia:magit:marlin:kntqzsqt:r1") ? (lambda (x) (and x #t)))))

    (test-case "honest evidence-chain risk fusion never fabricates HIGH without Aitia/human/conflict evidence"
      (let* ((fuse (.get UiMagitScope fuse-evidence-risk))
             (offline (fuse 'offline 0 'none 'unreviewed #f))
             (clean-admit (fuse 'online-admit 0 'none 'unreviewed #f))
             (formal-pending (fuse 'online-admit 0 'model-present-unrun 'unreviewed #f))
             (aitia-hit (fuse 'online-findings 2 'none 'unreviewed #f))
             (fully-verified (fuse 'online-admit 0 'verified 'audited #f)))
        (check (.get offline risk) => 'unassessed)
        (check (.get clean-admit risk) => 'low)
        (check (.get formal-pending risk) => 'medium)
        (check (.get aitia-hit risk) => 'high)
        (check (.get fully-verified risk) => 'info)))

    (test-case "hard Scheme contracts enforce SPC <-> native Emacs (C-x g / C-x M-g / C-c g) dual bindings"
      ;; Positive: UiMeowScope and UiMagitScope both instantiate cleanly and pair SPC g g <-> C-x g.
      (let* ((leaders (.get UiMagitScope doom-leader-bindings))
             (gg (find (lambda (b)
                         (and (string=? (.get b prefix) "SPC g")
                              (string=? (.get b key) "g")))
                       leaders))
             (meow-bindings (.get UiMeowScope bindings)))
        (check (.get gg emacs-key) => "C-x g")
        (check (find (lambda (e) (and (equal? (cadr e) '(C-x g)) (eq? (caddr e) 'magit-status)))
                     meow-bindings)
               ? (lambda (x) (and x #t)))
        (check (find (lambda (e) (and (equal? (cadr e) '(SPC g g)) (eq? (caddr e) 'magit-status)))
                     meow-bindings)
               ? (lambda (x) (and x #t))))
      ;; Negative control 1: UiMeowScopeContract MUST reject an orphan SPC binding without native C-*/M-*.
      (check-exception
        (make-marlin-ui-keymap '((normal (SPC z z) orphan-action "Missing native Emacs chord")))
        true)
      ;; Negative control 2: UiMagitScopeContract MUST reject a Doom SPC g binding lacking C-*/M-* emacs-key.
      (check-exception
        (.new/checked UiMagitScopeContract
          identity: 'marlin/ui-magit
          doom-leader-bindings: (list (.o prefix: "SPC g" key: "x" emacs-key: "bad-key"
                                          label: "Bad" column: "Bad" action: "cmd:bad" exit: #t))
          magit-transients: '()
          make-state: (.get UiMagitScope make-state)
          toggle-fold: (.get UiMagitScope toggle-fold)
          set-visibility-level: (.get UiMagitScope set-visibility-level)
          step-cursor: (.get UiMagitScope step-cursor)
          mark-hunk-audit: (.get UiMagitScope mark-hunk-audit)
          audit-progress: (.get UiMagitScope audit-progress)
          sort-hunks-by-risk: (.get UiMagitScope sort-hunks-by-risk)
          fuse-evidence-risk: (.get UiMagitScope fuse-evidence-risk)
          build-ai-audit-prompt: (.get UiMagitScope build-ai-audit-prompt)
          build-ai-commit-prompt: (.get UiMagitScope build-ai-commit-prompt)
          transient-for: (.get UiMagitScope transient-for))
        true))

    (test-case "native FFI projection exports Scheme commands, dual keymap, and all 14 Magit transients"
      (let* ((surf (marlin-ui-magit-project-grpc-surface #f))
             (cmds (vector-ref surf 0))
             (kmap (vector-ref surf 1))
             (trs  (vector-ref surf 2)))
        (check (vector-length surf) => 3)
        (check (> (vector-length cmds) 10) => #t)
        (check (> (vector-length kmap) 50) => #t)
        (check (vector-length trs) => 14)
        (check (vector-ref (vector-ref trs 0) 0) => "magit:?")))))
