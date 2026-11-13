;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Tool declarations and governance belong to the selected Scheme Module.
(.def Tooling
  identity: 'tooling
  core: (.o requires: (.o)
            scope-contract: ToolingScopeContract
            exports: ToolingScope)
  features: (.o))
