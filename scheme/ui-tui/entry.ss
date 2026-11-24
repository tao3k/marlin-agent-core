;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Linked entry to the selected UI scope. The workbench does not force Lab
;;; Context or other agent services as a side effect of its first frame.
(import (only-in :marlin-agent-core/c-abi/roots marlin-rooted-call)
        (only-in :marlin-agent-core/ui/config UiScope)
        (for-template :marlin-agent-core/ui-meow/interface)
        :marlin-agent-core/c-abi/ui
        :marlin-agent-core/c-abi/tooling
        :marlin-agent-core/c-abi/envrc
        :marlin-agent-core/c-abi/artifacts
        :marlin-agent-core/c-abi/worktree-context)
(export main)

(def (main . _) (void))
(def (marlin-tui-publication _) UiScope)

(begin-foreign
  (c-define (ui-publication root) (int64) int64
    "marlin_ui_publication" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/ui-tui/entry#marlin-tui-publication)))
