;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .def seal-object!)
        (only-in :clan/poo/mop .new/checked)
        "./syntax.ss" "./funcs.ss" "./types.ss")
(export UiHydraScope)

;;; The command catalog, transient bindings and hint come from this declaration.
(defui-hydra HistoryHydraValue
  (history color: red)
  "History"
  (p (history "Return to newest history" history "newest") "Newest")
  (n (history-older "Browse earlier history" history "older") "Older")
  (q #f "Quit" column: "Actions"))
(defui-hydra BuffersHydraValue
  (buffers color: blue key: (SPC b g))
  "Buffers"
  (c buffer-conversation "Conversation")
  (m buffer-messages "Messages")
  (h buffer-help "Help")
  (p buffer-processes "Processes")
  (q #f "Quit" column: "Actions"))
(.def UiHydraCatalog. entries: (list HistoryHydraValue BuffersHydraValue))
(def HydraCatalogValue (seal-object! UiHydraCatalog.))

(def UiHydraScope
  (.new/checked UiHydraScopeContract
    identity: 'marlin/ui-hydra
    catalog: HydraCatalogValue
    key-bindings: (marlin-ui-hydra-key-bindings HydraCatalogValue)
    command-entries: (lambda () (marlin-ui-hydra-command-entries HydraCatalogValue))
    entries: (lambda () (marlin-ui-hydra-entries HydraCatalogValue))))
