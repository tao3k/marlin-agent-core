;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Declarative transient command groups and their host-neutral projection.
(.def UiHydra
  identity: 'ui-hydra
  core: (.o requires: (.o UiElements: (.o module-id: 'ui-elements))
            scope-contract: UiHydraScopeContract
            exports: UiHydraScope)
  features: (.o))
