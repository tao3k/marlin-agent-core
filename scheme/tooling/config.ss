;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; The host admits this finite projection before advertising a tool. Adding
;;; another tool is a Scheme declaration, not a Rust command dispatcher case.
(import (only-in :clan/poo/object .o)
        (only-in :clan/poo/mop validate)
        "./types.ss" "./objects.ss")
(export ToolingScope)

(def ToolingScope
  (validate ToolingScopeContract
            (.o identity: 'marlin/tooling
                project: (lambda () (project-tooling ToolingCatalogValue)))))
