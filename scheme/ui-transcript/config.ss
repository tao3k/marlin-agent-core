;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .o)
        (only-in :clan/poo/mop validate)
        "./objects.ss" "./types.ss")
(export UiTranscriptScope)

(def UiTranscriptScope
  (validate UiTranscriptScopeContract
            (.o identity: 'marlin/ui-transcript
                select: marlin-ui-transcript-policy-for)))
