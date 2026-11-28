;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Semantic transcript presentation. Runtime events and ORG evidence remain
;;; with their owners; this module only selects labels and fold defaults.
(import (only-in :clan/poo/object .def seal-object!))
(export doom-transcript-policy pi-transcript-policy)

(.def UiTranscriptDoom.
  thinking-collapsed: #t
  user-prefix: "› "
  assistant-role: ""
  role-tone: 'info
  role-bold: #t
  thinking-collapsed-role: "▸ Thought"
  thinking-expanded-role: "▾ Thought")

(.def (UiTranscriptPi. @ UiTranscriptDoom.)
  role-tone: 'accent
  thinking-collapsed-role: "⋯"
  thinking-expanded-role: "⋯ Thought")

(def doom-transcript-policy (seal-object! UiTranscriptDoom.))
(def pi-transcript-policy (seal-object! UiTranscriptPi.))
