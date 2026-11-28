;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/mop .new/checked)
        (only-in "./objects.ss" DoomGitLeaderBindings MagitTransients)
        "./funcs.ss"
        "./types.ss")
(export UiMagitScope)

(def UiMagitScope
  (.new/checked UiMagitScopeContract
    identity: 'marlin/ui-magit
    doom-leader-bindings: DoomGitLeaderBindings
    magit-transients: MagitTransients
    make-state: marlin-magit-make-state
    toggle-fold: marlin-magit-toggle-fold
    set-visibility-level: marlin-magit-set-visibility-level
    step-cursor: marlin-magit-step-cursor
    mark-hunk-audit: marlin-magit-mark-hunk-audit
    audit-progress: marlin-magit-audit-progress
    sort-hunks-by-risk: marlin-magit-sort-hunks-by-risk
    fuse-evidence-risk: marlin-magit-fuse-evidence-risk
    build-ai-audit-prompt: marlin-magit-build-ai-audit-prompt
    build-ai-commit-prompt: marlin-magit-build-ai-commit-prompt
    transient-for: marlin-magit-transient-for))
