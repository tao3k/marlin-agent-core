;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Qualification image for the production mailbox transport.
(import (only-in :marlin-agent-core/c-abi/roots marlin-rooted-call)
        :marlin-agent-core/c-abi/deck-runtime
        (only-in "./mailbox.ss" run-mailbox-benchmark)
        (only-in "./context-profile.ss" run-context-profile run-context-retention-profile))
(export main)
(def (main . _) (void))
(begin-foreign
  (c-define (mailbox-run root) (int64) int64 "marlin_deck_mailbox_benchmark" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call
      root marlin-agent-core/bench/mailbox#run-mailbox-benchmark)))

(begin-foreign
  (c-define (context-profile root) (int64) int64 "marlin_deck_context_profile" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call
      root marlin-agent-core/bench/context-profile#run-context-profile)))

(begin-foreign
  (c-define (context-retention root) (int64) int64 "marlin_deck_context_retention" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call
      root marlin-agent-core/bench/context-profile#run-context-retention-profile)))
