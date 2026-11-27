;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Authentic Doom Emacs SPC g leader bindings paired 1:1 with native Emacs
;;; Magit chords (C-x g / C-x M-g / C-c g / C-c M-g) and full Magit transients
;;; derived directly from Magit's canonical lisp/magit*.el definitions.
(import (only-in :clan/poo/object .o))
(export DoomGitLeaderBindings
        MagitTransients
        MagitRiskOrder)

(def MagitRiskOrder
  '((high . 4)
    (medium . 3)
    (unassessed . 2)
    (low . 1)
    (info . 0)))

;; Authentic Doom Emacs SPC g (+git / Magit) leader bindings with mandatory
;; native Emacs chord pairing (enforced by UiMagitScopeContract).
(def DoomGitLeaderBindings
  (list
    (.o prefix: "SPC g" key: "g" emacs-key: "C-x g" label: "Magit status (*magit*)" column: "Status" action: "cmd:magit-status" exit: #t)
    (.o prefix: "SPC g" key: "G" emacs-key: "C-c M-g" label: "Magit refresh & Aitia audit" column: "Status" action: "cmd:magit-refresh" exit: #t)
    (.o prefix: "SPC g" key: "d" emacs-key: "C-x M-g" label: "Magit dispatch (?)" column: "Status" action: "hydra:magit:?" exit: #f)
    (.o prefix: "SPC g" key: "s" emacs-key: "C-c g s" label: "Stage file or hunk" column: "Changes" action: "cmd:magit-stage" exit: #t)
    (.o prefix: "SPC g" key: "u" emacs-key: "C-c g u" label: "Unstage file or hunk" column: "Changes" action: "cmd:magit-unstage" exit: #t)
    (.o prefix: "SPC g" key: "c" emacs-key: "C-c g c" label: "Commit / jj describe" column: "Commit" action: "hydra:magit:c" exit: #f)
    (.o prefix: "SPC g" key: "b" emacs-key: "C-c g b" label: "Branch / jj bookmark" column: "Branch" action: "hydra:magit:b" exit: #f)
    (.o prefix: "SPC g" key: "l" emacs-key: "C-c g l" label: "Log / revset / op log" column: "History" action: "hydra:magit:l" exit: #f)
    (.o prefix: "SPC g" key: "r" emacs-key: "C-c g r" label: "Rebase / squash / AI split" column: "Rewrite" action: "hydra:magit:r" exit: #f)
    (.o prefix: "SPC g" key: "f" emacs-key: "C-c g f" label: "Fetch remotes" column: "Sync" action: "hydra:magit:f" exit: #f)
    (.o prefix: "SPC g" key: "F" emacs-key: "C-c g F" label: "Pull from upstream" column: "Sync" action: "hydra:magit:F" exit: #f)
    (.o prefix: "SPC g" key: "P" emacs-key: "C-c g P" label: "Push to remote" column: "Sync" action: "hydra:magit:P" exit: #f)
    (.o prefix: "SPC g" key: "z" emacs-key: "C-c g z" label: "Stash / jj undo" column: "Safety" action: "hydra:magit:z" exit: #f)
    (.o prefix: "SPC g" key: "m" emacs-key: "C-c g m" label: "Merge / AI resolve" column: "Safety" action: "hydra:magit:m" exit: #f)
    (.o prefix: "SPC g" key: "a" emacs-key: "C-c g" label: "AI Code Audit (50/50 split)" column: "AI Audit" action: "cmd:magit-ai-audit" exit: #t)))

;; Canonical Magit transient menus synced from magit.el / magit-{commit,diff,branch,
;; log,stash,fetch,pull,push,reset,sequence,merge}.el, with AI & Aitia nested cleanly.
(def MagitTransients
  (list
    (.o prefix: "magit:?" id: "magit-dispatch" title: "Magit Dispatch (? / C-x M-g)"
        heads: (list
                 (.o key: "c" label: "Commit / jj describe" column: "Transient" action: "hydra:magit:c" exit: #f)
                 (.o key: "d" label: "Diff & Aitia/AI Audit" column: "Transient" action: "hydra:magit:d" exit: #f)
                 (.o key: "b" label: "Branch / jj bookmark" column: "Transient" action: "hydra:magit:b" exit: #f)
                 (.o key: "l" label: "Log / jj op log" column: "Transient" action: "hydra:magit:l" exit: #f)
                 (.o key: "f" label: "Fetch" column: "Sync" action: "hydra:magit:f" exit: #f)
                 (.o key: "F" label: "Pull" column: "Sync" action: "hydra:magit:F" exit: #f)
                 (.o key: "P" label: "Push" column: "Sync" action: "hydra:magit:P" exit: #f)
                 (.o key: "r" label: "Rebase / squash / AI split" column: "Rewrite" action: "hydra:magit:r" exit: #f)
                 (.o key: "z" label: "Stash / jj undo" column: "Rewrite" action: "hydra:magit:z" exit: #f)
                 (.o key: "X" label: "Reset (soft/mixed)" column: "Rewrite" action: "hydra:magit:X" exit: #f)
                 (.o key: "A" label: "Cherry-pick" column: "Rewrite" action: "hydra:magit:A" exit: #f)
                 (.o key: "V" label: "Revert commit" column: "Rewrite" action: "hydra:magit:V" exit: #f)
                 (.o key: "m" label: "Merge / AI resolve" column: "Rewrite" action: "hydra:magit:m" exit: #f)
                 (.o key: "g" label: "Refresh & Aitia re-audit" column: "Buffer" action: "cmd:magit-refresh" exit: #t)))
    (.o prefix: "magit:c" id: "magit-commit" title: "Magit Commit (c)"
        heads: (list
                 (.o key: "c" label: "Commit staged" column: "Commit" action: "cmd:magit-commit-prompt" exit: #t)
                 (.o key: "a" label: "Amend HEAD" column: "Commit" action: "cmd:magit-amend-prompt" exit: #t)
                 (.o key: "e" label: "Extend HEAD (no edit)" column: "Commit" action: "cmd:magit-extend" exit: #t)
                 (.o key: "w" label: "Reword / jj describe (@)" column: "Jujutsu" action: "cmd:magit-describe-prompt" exit: #t)
                 (.o key: "n" label: "jj new (next change)" column: "Jujutsu" action: "cmd:magit-jj-new" exit: #t)
                 (.o key: "i" label: "AI Commit Draft (Aitia trailer)" column: "AI Native" action: "cmd:magit-ai-commit" exit: #t)))
    (.o prefix: "magit:d" id: "magit-diff" title: "Magit Diff, Aitia & AI Audit (d)"
        heads: (list
                 (.o key: "a" label: "AI Audit selected hunk (50/50 split)" column: "AI Audit" action: "cmd:magit-ai-audit" exit: #t)
                 (.o key: "r" label: "AI Full Evidence-Chain Triage" column: "AI Audit" action: "cmd:magit-ai-triage" exit: #t)
                 (.o key: "A" label: "Re-run Lambda Aitia gRPC Defense Audit" column: "Aitia" action: "cmd:magit-refresh" exit: #t)
                 (.o key: "d" label: "Expand full diff (Level 4)" column: "View" action: "cmd:magit-level-4" exit: #t)
                 (.o key: "s" label: "Hunks summary (Level 3)" column: "View" action: "cmd:magit-level-3" exit: #t)))
    (.o prefix: "magit:b" id: "magit-branch" title: "Magit Branch / jj Bookmark (b)"
        heads: (list
                 (.o key: "b" label: "Checkout branch" column: "Branch" action: "cmd:magit-branch-prompt" exit: #t)
                 (.o key: "c" label: "Create & checkout branch" column: "Branch" action: "cmd:magit-branch-create-prompt" exit: #t)
                 (.o key: "k" label: "Delete branch" column: "Branch" action: "cmd:magit-branch-delete-prompt" exit: #t)))
    (.o prefix: "magit:f" id: "magit-fetch" title: "Magit Fetch (f)"
        heads: (list
                 (.o key: "u" label: "Fetch from upstream" column: "Fetch" action: "cmd:magit-fetch-upstream" exit: #t)
                 (.o key: "a" label: "Fetch all remotes" column: "Fetch" action: "cmd:magit-fetch-all" exit: #t)))
    (.o prefix: "magit:F" id: "magit-pull" title: "Magit Pull (F)"
        heads: (list
                 (.o key: "u" label: "Pull from upstream" column: "Pull" action: "cmd:magit-pull" exit: #t)
                 (.o key: "r" label: "Pull --rebase" column: "Pull" action: "cmd:magit-pull-rebase" exit: #t)))
    (.o prefix: "magit:P" id: "magit-push" title: "Magit Push (P)"
        heads: (list
                 (.o key: "p" label: "Push to upstream" column: "Push" action: "cmd:magit-push" exit: #t)))
    (.o prefix: "magit:l" id: "magit-log" title: "Magit Log & jj Op Log (l)"
        heads: (list
                 (.o key: "l" label: "Refresh & focus log" column: "History" action: "cmd:magit-refresh" exit: #t)
                 (.o key: "o" label: "Toggle jj Operation Log" column: "History" action: "cmd:magit-toggle-oplog" exit: #t)))
    (.o prefix: "magit:r" id: "magit-rewrite" title: "Magit Rebase / Squash / AI Split (r)"
        heads: (list
                 (.o key: "q" label: "jj squash into parent" column: "Rewrite" action: "cmd:magit-jj-squash" exit: #t)
                 (.o key: "s" label: "AI Semantic Split Proposal" column: "AI Native" action: "cmd:magit-ai-split" exit: #t)
                 (.o key: "n" label: "jj new child change" column: "Rewrite" action: "cmd:magit-jj-new" exit: #t)))
    (.o prefix: "magit:z" id: "magit-stash" title: "Magit Stash & jj Time-Travel (z)"
        heads: (list
                 (.o key: "z" label: "Stash both (index + worktree)" column: "Stash" action: "cmd:magit-stash-push" exit: #t)
                 (.o key: "p" label: "Pop latest stash" column: "Stash" action: "cmd:magit-stash-pop" exit: #t)
                 (.o key: "a" label: "Apply latest stash" column: "Stash" action: "cmd:magit-stash-apply" exit: #t)
                 (.o key: "k" label: "Drop latest stash" column: "Stash" action: "cmd:magit-stash-drop" exit: #t)
                 (.o key: "u" label: "jj undo (revert last op)" column: "Jujutsu" action: "cmd:magit-jj-undo" exit: #t)))
    (.o prefix: "magit:X" id: "magit-reset" title: "Magit Reset (X)"
        heads: (list
                 (.o key: "s" label: "Soft reset HEAD~1 (keep staged)" column: "Reset" action: "cmd:magit-reset-soft" exit: #t)
                 (.o key: "m" label: "Mixed reset HEAD~1 (unstage)" column: "Reset" action: "cmd:magit-reset-mixed" exit: #t)))
    (.o prefix: "magit:A" id: "magit-cherry-pick" title: "Magit Cherry-Pick (A)"
        heads: (list
                 (.o key: "A" label: "Cherry-pick revision..." column: "Apply" action: "cmd:magit-cherry-pick-prompt" exit: #t)))
    (.o prefix: "magit:V" id: "magit-revert" title: "Magit Revert (V)"
        heads: (list
                 (.o key: "V" label: "Revert revision..." column: "Revert" action: "cmd:magit-revert-prompt" exit: #t)))
    (.o prefix: "magit:m" id: "magit-merge" title: "Magit Merge & AI Conflict Resolve (m)"
        heads: (list
                 (.o key: "a" label: "AI 3-Way Conflict Resolution" column: "AI Native" action: "cmd:magit-ai-resolve" exit: #t)))))
