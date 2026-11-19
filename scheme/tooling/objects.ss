;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; A catalog can be extended through POO composition; the projection shape
;;; remains stable for any host that implements the declared capability.
(import (only-in :clan/poo/object .def .mix seal-object!))
(export ToolingCatalogValue)

(.def ToolDeclaration.
  name: 'bash
  description: "Run a Bash command in the workspace after approval."
  capability: 'shell
  governance: 'ask
  argument: 'command
  timeout-seconds: 30
  output-bytes: 65536)
(def BashToolDeclaration (seal-object! ToolDeclaration.))
(.def ToolingCatalog. declarations: '())
(.def BashTools.
  (declarations => append (list BashToolDeclaration)))
(def ToolingCatalogValue
  (seal-object! (.mix BashTools. ToolingCatalog.)))
