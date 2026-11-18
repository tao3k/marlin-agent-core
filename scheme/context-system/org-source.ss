;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Source-backed Org Elements use Lab's Scheme Orgize interface.
(import (only-in :clan/poo/object .o)
        (only-in :tao3k/agentic-ai-lab/modules/query/semantic/orgize-source-interface
                 org-source-element-graph))
(export marlin-context-org-source-graph)

(def (marlin-context-org-source-graph source-value)
  ;; Keep the complete source alongside the derived graph. Graph selection is
  ;; not a journal, authority grant, or a replacement for the source bytes.
  (let* ((owned-source (string-copy source-value))
         (graph-value (org-source-element-graph owned-source)))
    (.o kind: 'marlin.context.org-source-graph.v1
        source: owned-source graph: graph-value)))
