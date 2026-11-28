;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Declarative transient command groups and their host-neutral projection.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" UiHydraScope)
        (only-in "./types.ss" UiHydraScopeContract))
(export UiHydra)

(.def UiHydra
  identity: 'ui-hydra
  core: (.o scope-contract: UiHydraScopeContract
            exports: UiHydraScope))
