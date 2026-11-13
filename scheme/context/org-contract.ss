;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Publish the Lab-owned Org Contract source cut without a Rust Org parser.
(import (only-in :clan/poo/object .ref)
        (only-in :tao3k/agentic-ai-lab/modules/ai-agentic-context/org-contract/elements
                 context-org-source))
(export marlin-context-org-contract-source)

(def (marlin-context-org-contract-source request)
  (context-org-source (.ref request 'identity) (.ref request 'generation)
                      (.ref request 'source)))
