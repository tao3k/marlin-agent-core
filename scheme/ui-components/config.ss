;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .get)
        (only-in :clan/poo/type element?)
        (only-in "../ui-hydra/config.ss" UiHydraScope)
        (only-in "../ui-hydra/types.ss" UiHydraScopeContract)
        (only-in "./objects.ss" make-ui-components-operations))
(export make-ui-components-scope UiComponentsScope)
(def (make-ui-components-scope hydra)
  (unless (element? UiHydraScopeContract hydra) (error "invalid selected hydra scope"))
  (make-ui-components-operations
    (.get hydra command-entries) (.get hydra entries) (.get hydra key-bindings)))

(def UiComponentsScope (make-ui-components-scope UiHydraScope))
