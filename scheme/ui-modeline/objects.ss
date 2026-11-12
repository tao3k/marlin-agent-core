;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Declarative modeline content and semantic tones. The host resolves live
;;; model, session and activity facts without a VM call on each frame.
(import (only-in :clan/poo/object .def .get .mix .o seal-object!))
(export marlin-ui-modeline-nodes)

(.def UiModelineBase.
  slots: '((modeline/left/brand "🐟 MARLIN" "🐟" accent accent)
           (modeline/left/model "" "" info info)
           (modeline/left/activity "Gateway configured" "Ready" success warning)))
(.def UiModelineSession.
  (slots => append '((modeline/right/session "" "" muted muted))))
(def modeline-catalog
  (seal-object! (.mix UiModelineSession. UiModelineBase.)))

(def (marlin-ui-modeline-nodes)
  (map (lambda (entry)
         (.o identity: (car entry) label: (cadr entry)
             compact: (caddr entry) tone: (cadddr entry)
             active-tone: (list-ref entry 4)))
       (.get modeline-catalog slots)))
