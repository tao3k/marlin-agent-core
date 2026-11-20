;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
(import (only-in :clan/poo/object .def .o))
(export Build)

(.def Build
  defbuild-script: (.o spec: '("entry"))
  autoload:
  (.o workflow:
      (.o module: ':marlin-agent-core/brief-to-draft/config
          exports: (.o validate-settings: 'validate-settings
                       propose: 'propose))))
