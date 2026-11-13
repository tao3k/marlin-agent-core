;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .def .o seal-object!)
        (only-in :clan/poo/mop validate)
        "./objects.ss" "./types.ss")
(export UiHydraScope)

;;; The command catalog, transient bindings and hint come from this declaration.
(defui-hydra HistoryHydraValue
  (history color: red hint: #f)
  "\n^History^\n_p_: newest  _n_: older  _q_: quit"
  (p (history "Return to newest history" history "newest"))
  (n (history-older "Browse earlier history" history "older"))
  (q #f))
(defui-hydra BuffersHydraValue
  (buffers color: blue)
  "Buffers"
  (c buffer-conversation "conversation")
  (m buffer-messages "messages")
  (h buffer-help "help")
  (p buffer-processes "processes")
  (q #f "quit"))
(.def UiHydraCatalog. entries: (list HistoryHydraValue BuffersHydraValue))
(def HydraCatalogValue (seal-object! UiHydraCatalog.))

(def UiHydraScope
  (validate UiHydraScopeContract
            (.o identity: 'marlin/ui-hydra
                catalog: HydraCatalogValue
                command-entries: (lambda () (marlin-ui-hydra-command-entries HydraCatalogValue))
                entries: (lambda () (marlin-ui-hydra-entries HydraCatalogValue)))))
