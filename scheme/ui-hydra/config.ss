;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .def .o seal-object!)
        (only-in :clan/poo/mop validate)
        "./objects.ss" "./types.ss")
(export UiHydraScope)

;;; The command catalog, transient bindings and hint come from this declaration.
(defui-hydra HistoryHydraValue
  (history (C-x h) color: red)
  "History"
  (p (history "Return to newest history" history "newest") "newest")
  (n (history-older "Browse earlier history" history "older") "older")
  (q #f "quit" color: blue))
(.def UiHydraCatalog. entries: (list HistoryHydraValue))
(def HydraCatalogValue (seal-object! UiHydraCatalog.))

(def UiHydraScope
  (validate UiHydraScopeContract
            (.o identity: 'marlin/ui-hydra
                catalog: HydraCatalogValue
                command-entries: (lambda () (marlin-ui-hydra-command-entries HydraCatalogValue))
                keymap-entries: (lambda () (marlin-ui-hydra-keymap-entries HydraCatalogValue)))))
