;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Context operations provided by Marlin to its runtime boundary.

(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" MarlinContextSystemScope)
        (only-in "./objects.ss" EnableCausalEvidence)
        (only-in "./types.ss" MarlinContextSystemContract ContextCausalContract))
(export ContextSystem)

(.def ContextSystem
  identity: 'context-system
  core: (.o scope-contract: MarlinContextSystemContract
            exports: MarlinContextSystemScope)
  features: (.o causal: (.o refine: EnableCausalEvidence contract: ContextCausalContract)))
