;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import :std/test
        (only-in :clan/poo/object .get .o)
        (only-in :clan/poo/mop .new/checked)
        (only-in "../ui-modes/config.ss" UiModesScope)
        (only-in "../ui-modes/types.ss" UiModesScopeContract))
(export ui-modes-test)

(def ui-modes-test
  (test-suite "Marlin Emacs Major/Minor Modes, Hooks & AI-Native Polymode"
    (test-case "major-mode parent inheritance chain resolves authentically"
      (let ((chain ((.get UiModesScope parent-chain) UiModesScope 'magit-status-mode)))
        (check chain => '(magit-status-mode magit-mode marlin-special-mode fundamental-mode))))

    (test-case "buffer-to-mode routing handles multi-instance *chat:<id>* and *magit*"
      (let ((resolve (.get UiModesScope resolve-buffer-mode)))
        (check (resolve UiModesScope "*chat:refactor-auth*") => 'marlin-chat-mode)
        (check (resolve UiModesScope "*conversation*") => 'marlin-chat-mode)
        (check (resolve UiModesScope "*magit*") => 'magit-status-mode)
        (check (resolve UiModesScope "*Agent Room*") => 'marlin-room-mode)
        (check (resolve UiModesScope "*Dashboard*") => 'marlin-dashboard-mode)
        (check (resolve UiModesScope "*Unknown*") => 'fundamental-mode)))

    (test-case "Polymode switches inner mode by span and fires hooks only on transitions"
      (let* ((b0 ((.get UiModesScope make-buffer-state) UiModesScope "*chat:design*"))
             ;; Cursor enters an AI-proposed diff hunk inside *chat:design*!
             (b1 ((.get UiModesScope switch-span) UiModesScope #f b0 'diff-hunk))
             (st1 ((.get UiModesScope effective-stack) UiModesScope b1 'diff-hunk))
             ;; Same span kind again -> must NOT fire duplicate switch hook.
             (b2 ((.get UiModesScope switch-span) UiModesScope #f b1 'diff-hunk))
             ;; Cursor moves into a tool approval gate span.
             (b3 ((.get UiModesScope switch-span) UiModesScope #f b2 'tool-gate))
             (st3 ((.get UiModesScope effective-stack) UiModesScope b3 'tool-gate)))
        (check (.get b1 inner-mode) => 'magit-diff-inner-mode)
        (check (.get st1 display-title) => "Marlin-Chat[Diff·Aitia]")
        (check (length (.get b2 hook-log)) => (length (.get b1 hook-log)))
        (check (.get b3 inner-mode) => 'marlin-gate-inner-mode)
        (check (.get st3 display-title) => "Marlin-Chat[Gate]")))

    (test-case "Emacs keymap lookup precedence: minor-mode > polymode inner > major-mode > parent"
      (let* ((chat0 ((.get UiModesScope make-buffer-state) UiModesScope "*chat:audit*"))
             (lookup (.get UiModesScope lookup-key))
             ;; In prose span: host marlin-chat-mode binds "i" -> cmd:insert, aitia minor mode binds C-c g.
             (prose-i (lookup UiModesScope chat0 'prose "i"))
             (prose-cg (lookup UiModesScope chat0 'prose "C-c g"))
             (prose-s (lookup UiModesScope chat0 'prose "s"))
             ;; In diff-hunk span inside chat: Polymode magit-diff-inner-mode activates "s" -> cmd:magit-stage!
             (diff-s (lookup UiModesScope chat0 'diff-hunk "s"))
             (diff-a (lookup UiModesScope chat0 'diff-hunk "a"))
             ;; In tool-gate span inside chat: Polymode marlin-gate-inner-mode activates "y" -> cmd:gate-approve!
             (gate-y (lookup UiModesScope chat0 'tool-gate "y"))
             ;; In *magit*: inherits "q" from marlin-special-mode parent while overriding "g" in magit-mode!
             (magit0 ((.get UiModesScope make-buffer-state) UiModesScope "*magit*"))
             (magit-q (lookup UiModesScope magit0 'prose "q"))
             (magit-g (lookup UiModesScope magit0 'prose "g")))
        (check (.get prose-i command) => "cmd:insert")
        (check (.get prose-cg command) => "cmd:magit-ai-audit")
        (check prose-s => #f)
        (check (.get diff-s command) => "cmd:magit-stage")
        (check (.get diff-a command) => "cmd:magit-ai-audit")
        (check (.get gate-y command) => "cmd:gate-approve")
        (check (.get magit-q command) => "cmd:win-delete")
        (check (.get magit-g command) => "cmd:magit-refresh")))

    (test-case "minor-mode toggle fires enter/exit hooks deterministically"
      (let* ((b0 ((.get UiModesScope make-buffer-state) UiModesScope "*chat:zen*"))
             (toggle (.get UiModesScope toggle-minor-mode))
             (b1 (toggle UiModesScope #f b0 'zen-writeroom-mode))
             (b2 (toggle UiModesScope #f b1 'zen-writeroom-mode)))
        (check (and (memq 'zen-writeroom-mode (.get b1 minor-modes)) #t) => #t)
        (check (and (memq 'hide-chrome-zen (.get b1 hook-log)) #t) => #t)
        (check (and (memq 'zen-writeroom-mode (.get b2 minor-modes)) #t) => #f)
        (check (and (memq 'restore-chrome-zen (.get b2 hook-log)) #t) => #t)))

    (test-case "hard POO contract rejects cyclic major-mode inheritance"
      (check-exception
        (.new/checked UiModesScopeContract
          identity: 'marlin/ui-modes
          major-modes: (list
                         (.o id: 'mode-a title: "A" parent: 'mode-b read-only: #f
                             display-action: 'reuse-window dedicated: #f local-keymap: '()
                             default-minor-modes: '() modeline-segments: '()
                             new-buffer-actions: '() enter-hooks: '() exit-hooks: '())
                         (.o id: 'mode-b title: "B" parent: 'mode-a read-only: #f
                             display-action: 'reuse-window dedicated: #f local-keymap: '()
                             default-minor-modes: '() modeline-segments: '()
                             new-buffer-actions: '() enter-hooks: '() exit-hooks: '()))
          minor-modes: '()
          inner-modes: '()
          polymodes: '()
          buffer-mode-rules: '()
          parent-chain: (.get UiModesScope parent-chain)
          resolve-buffer-mode: (.get UiModesScope resolve-buffer-mode)
          resolve-polymode-inner: (.get UiModesScope resolve-polymode-inner)
          make-buffer-state: (.get UiModesScope make-buffer-state)
          effective-stack: (.get UiModesScope effective-stack)
          lookup-key: (.get UiModesScope lookup-key)
          switch-span: (.get UiModesScope switch-span)
          toggle-minor-mode: (.get UiModesScope toggle-minor-mode)
          project-grpc-registry: (.get UiModesScope project-grpc-registry))
        true))

    (test-case "native FFI projection emits 5-vector registry with zero text serialization"
      (let ((proj ((.get UiModesScope project-grpc-registry) #f)))
        (check (vector-length proj) => 5)
        (check (>= (vector-length (vector-ref proj 0)) 14) => #t)
        (check (>= (vector-length (vector-ref proj 1)) 6) => #t)
        (check (= (vector-length (vector-ref proj 2)) 4) => #t)
        (check (= (vector-length (vector-ref proj 3)) 3) => #t)
        (check (>= (vector-length (vector-ref proj 4)) 12) => #t)))))
