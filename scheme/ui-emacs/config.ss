;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/mop .new/checked)
        "./objects.ss"
        "./funcs.ss"
        "./types.ss")
(export UiEmacsScope)

(def UiEmacsScope
  (.new/checked UiEmacsScopeContract
    identity: 'marlin/ui-emacs
    default-consult-sources: DefaultConsultSources
    make-session: marlin-emacs-make-session
    bookmark-set: marlin-emacs-bookmark-set
    bookmark-jump: marlin-emacs-bookmark-jump
    bookmark-delete: marlin-emacs-bookmark-delete
    bookmark-list: marlin-emacs-bookmark-list
    kill-push: marlin-emacs-kill-push
    yank-current: marlin-emacs-yank-current
    yank-pop: marlin-emacs-yank-pop
    jump-push: marlin-emacs-jump-push
    jump-back: marlin-emacs-jump-back
    jump-forward: marlin-emacs-jump-forward
    push-mark: marlin-emacs-push-mark
    pop-global-mark: marlin-emacs-pop-global-mark
    add-hook: marlin-emacs-add-hook
    remove-hook: marlin-emacs-remove-hook
    run-hooks: marlin-emacs-run-hooks
    project-consult-sources: marlin-emacs-project-consult-sources))
