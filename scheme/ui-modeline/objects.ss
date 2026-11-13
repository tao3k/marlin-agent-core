;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Declarative modeline content and semantic tones. The host resolves live
;;; model, session and activity facts without a VM call on each frame.
(import (only-in :clan/poo/object .def .get .mix .o seal-object!))
(export marlin-ui-modeline-nodes marlin-ui-modeline-nodes-for)

(.def UiModelineBase.
  activity-separator: "│ "
  slots: '((modeline/left/brand "🐟 MARLIN" "🐟" accent accent 0 18 0)
           (modeline/left/model "  {value}  " "{value}" info info 0 0 3)
           (modeline/left/activity "🟡 Ready" "🟡" info warning 0 64 0)))
(.def UiModelineSession.
  (slots => append '((modeline/right/turns "⟳{value}  " "⟳{value}" info info 72 0 1)
                     (modeline/right/session " ◉ {value} " "◉{value}" muted muted 44 0 2)
                     (modeline/state/online "🟢 Online" "🟢" success success 0 64 0)
                     (modeline/state/offline "🔴 Offline" "🔴" warning warning 0 64 0)
                     (modeline/state/unknown "🟠 Unknown" "🟠" warning warning 0 64 0)
                     (modeline/state/runtime "⚙ Runtime" "⚙" info info 0 64 0)
                     (modeline/state/workspace "⌁ Workspace" "⌁" info info 0 64 0)
                     (modeline/state/recovery "🟠 Recovery" "🟠" warning warning 0 64 0)
                     (modeline/state/continuation "⚠ Continue blocked" "⚠" warning warning 0 64 0)
                     (modeline/state/history "◷ History" "◷" muted muted 0 64 0)
                     (modeline/state/working "◌ Working" "◌" info info 0 64 0)
                     (modeline/state/reading "⌁ Reading" "⌁" info info 0 64 0))))
(def modeline-catalog
  (seal-object! (.mix UiModelineSession. UiModelineBase.)))

(def pi-modeline-catalog
  (seal-object! (.o activity-separator: "· "
                    slots: '((modeline/left/brand "🐟 marlin" "🐟" info info 0 16 0)
                              (modeline/left/model "  {value}  " "{value}" accent accent 0 0 1)
                              (modeline/left/activity "● ready" "●" success warning 0 48 0)
                              (modeline/right/session " ◉ {value} " "◉{value}" muted muted 40 0 3)
                              (modeline/right/turns "⟳{value}  " "⟳{value}" info info 66 0 2)
                              (modeline/state/online "● online" "●" success success 0 48 0)
                              (modeline/state/offline "● offline" "●" warning warning 0 48 0)
                              (modeline/state/unknown "● unknown" "●" warning warning 0 48 0)
                              (modeline/state/runtime "● runtime" "●" info info 0 48 0)
                              (modeline/state/workspace "● workspace" "●" info info 0 48 0)
                              (modeline/state/recovery "● recovery" "●" warning warning 0 48 0)
                              (modeline/state/continuation "● blocked" "●" warning warning 0 48 0)
                              (modeline/state/history "● history" "●" muted muted 0 48 0)
                              (modeline/state/working "● working" "●" info info 0 48 0)
                              (modeline/state/reading "● reading" "●" info info 0 48 0)))))

(def (marlin-ui-modeline-nodes-for profile)
  (def catalog (case profile
                 ((doom-one) modeline-catalog)
                 ((pi) pi-modeline-catalog)
                 (else (error "unknown UI modeline profile"))))
  (map (lambda (entry)
         (let* ((identity-value (car entry))
                (activity? (or (eq? identity-value 'modeline/left/activity)
                               (string-prefix? "modeline/state/" (symbol->string identity-value))))
                (prefix (if activity? (.get catalog activity-separator) "")))
           (.o identity: identity-value label: (string-append prefix (cadr entry))
             compact: (string-append prefix (caddr entry)) tone: (cadddr entry)
             active-tone: (list-ref entry 4)
             min-width: (list-ref entry 5) compact-below: (list-ref entry 6)
             keep-priority: (list-ref entry 7))))
       (.get catalog slots)))

(def (marlin-ui-modeline-nodes)
  (marlin-ui-modeline-nodes-for 'doom-one))
