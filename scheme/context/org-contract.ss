;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Publish the Lab-owned Org Contract source cut without a Rust Org parser.
(import (only-in :marlin-agent-core/interface LabContext)
        (only-in :clan/poo/object .ref .call))

(export marlin-context-org-contract-source)

(def (marlin-context-org-contract-source request)
  (.call LabContext org-source (.ref request 'identity) (.ref request 'generation)
         (.ref request 'source)))
