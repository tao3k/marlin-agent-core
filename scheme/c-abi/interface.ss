;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Native adapters are a selected Module; Rust owns their ABI readiness.
(.def CAbi
  identity: 'c-abi
  core: (.o scope-contract: CAbiScopeContract
            exports: CAbiScope
            projections: (list Build))
  features: (.o))
