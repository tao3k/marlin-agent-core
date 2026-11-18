;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Tool declarations and governance belong to the selected Scheme Module.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" ToolingScope)
        (only-in "./types.ss" ToolingScopeContract))
(export Tooling)

(.def Tooling
  identity: 'tooling
  core: (.o scope-contract: ToolingScopeContract
            exports: ToolingScope)
  features: (.o))
