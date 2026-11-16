;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Context operations provided by Marlin to its runtime boundary.

(.def Context
  identity: 'context
  core: (.o scope-contract: MarlinContextModuleContract
            exports: MarlinContextScope)
  features: (.o causal: (.o refine: EnableCausalEvidence contract: ContextCausalContract)))
