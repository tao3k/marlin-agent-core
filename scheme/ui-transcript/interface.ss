;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Profile-selected transcript labels; no runtime or session ownership.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" UiTranscriptScope)
        (only-in "./types.ss" UiTranscriptScopeContract))
(export UiTranscript)

(.def UiTranscript
  identity: 'ui-transcript
  core: (.o scope-contract: UiTranscriptScopeContract
            exports: UiTranscriptScope))
