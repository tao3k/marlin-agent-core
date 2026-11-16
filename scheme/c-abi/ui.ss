;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; C entry points are hosted by the linked program. Scheme packages use
;;; the session procedures directly, without requiring any host ABI symbol.
(import (only-in :marlin-agent-core/c-abi/roots marlin-rooted-call)
        :marlin-agent-core/ui/session)

(begin-foreign
  (c-define (ui-package-publication root) (int64) int64
    "marlin_ui_package_publication" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/ui/package#marlin-ui-load-publication)))

(begin-foreign
  (c-define (ui-open root) (int64) int64
    "marlin_ui_open" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/ui/session#marlin-ui-session-open)))
(begin-foreign
  (c-define (ui-project-workbench root) (int64) int64
    "marlin_ui_project_workbench" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/ui/session#marlin-ui-session-project)))
(begin-foreign
  (c-define (ui-dispatch-event root) (int64) int64
    "marlin_ui_dispatch_event" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/ui/session#marlin-ui-session-dispatch)))
(begin-foreign
  (c-define (ui-browse-command root) (int64) int64
    "marlin_ui_browse_command" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/ui/session#marlin-ui-session-browse-command)))
(begin-foreign
  (c-define (ui-command-candidates root) (int64) int64
    "marlin_ui_command_candidates" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/ui/session#marlin-ui-session-command-candidates)))
(begin-foreign
  (c-define (ui-review-menu root) (int64) int64
    "marlin_ui_review_menu" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/ui/session#marlin-ui-session-review-menu)))
(begin-foreign
  (c-define (ui-refresh-labels root) (int64) int64
    "marlin_ui_refresh_labels" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/ui/session#marlin-ui-session-refresh-labels)))
