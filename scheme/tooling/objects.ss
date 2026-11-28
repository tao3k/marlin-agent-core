;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; A catalog can be extended through POO composition; the projection shape
;;; remains stable for any host that implements the declared capability.
(import (only-in :clan/poo/object .def .get .mix .o seal-object!)
        (only-in "../search-dsl/objects.ss" WorkspaceSearchToolSpecValue))
(export ToolingCatalogValue)

(.def ToolDeclaration.
  name: 'bash
  description: "Run a Bash command in the workspace after approval (builds, tests, git, or file edits). Do NOT use bash for codebase search, rg/grep, find, or directory/file listing — ALWAYS use workspace_search first."
  capability: 'shell
  governance: 'ask
  argument: 'command
  timeout-seconds: 30
  output-bytes: 65536)
(def BashToolDeclaration (seal-object! ToolDeclaration.))
(def WorkspaceSearchToolDeclaration
  (seal-object!
    (.mix (.o name: 'workspace_search
              description: (.get WorkspaceSearchToolSpecValue summary)
              capability: 'search
              governance: 'ask
              argument: 'query
              timeout-seconds: 15
              output-bytes: 65536)
          ToolDeclaration.)))
(.def ToolingCatalog. declarations: '())
(.def BashTools.
  (declarations => append (list BashToolDeclaration)))
(.def SearchTools.
  (declarations => append (list WorkspaceSearchToolDeclaration)))
(def ToolingCatalogValue
  (seal-object! (.mix SearchTools. BashTools. ToolingCatalog.)))
