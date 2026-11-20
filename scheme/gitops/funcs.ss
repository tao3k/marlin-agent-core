;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/type validate)
        (only-in :clan/poo/object .cc .get .o)
        (only-in "./types.ss" WorkingCopyObservation))
(export select-host-working-copy-provider)

;; The Rust observation carries Git availability, Worktrunk availability and preference.
(def (select-host-working-copy-provider scope observation)
  (def request (validate WorkingCopyObservation observation))
  ((.get scope select-working-copy)
   (.cc (.get scope working-copy-policy)
        preferred: (if (vector-ref request 2) 'worktrunk 'git-worktree))
   (.o git-worktree: (vector-ref request 0) worktrunk: (vector-ref request 1))))
