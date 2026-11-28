;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .def .o)
        (only-in :tao3k/yuanxu/module-system/module-hooks ModuleHook.)
        (only-in "./funcs.ss" load-configured-publication))
(export SchemePackageHooks.)

(.def SchemePackageHooks.
  configured-ui:
  (.o (:: @ ModuleHook.)
      on: '(ui-command-execution ui-key-command)
      .run: load-configured-publication))
