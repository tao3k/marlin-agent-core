;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/mop Fun Object Type.)
        (only-in :clan/poo/type Exactly Record))
(export UiTranscriptScopeContract)

(def UiTranscriptProfile (.cc Type. '.element? symbol?))
(def UiTranscriptScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-transcript)]
               select: [(Fun Object <- UiTranscriptProfile)])
       identity: 'marlin/ui-transcript))
