;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" EnvrcScope)
        (only-in "./types.ss" EnvrcScopeContract))
(export Envrc)

(.def Envrc
  identity: 'envrc
  core: (.o scope-contract: EnvrcScopeContract
            exports: EnvrcScope))
