;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Rooted Scheme values only. Rust validates the shape and owns provider JSON.
(import (only-in :marlin-agent-core/c-abi/roots marlin-rooted-call)
        (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/tooling/config ToolingScope))
(export marlin-tooling-project)

(def (marlin-tooling-project _)
  ((.get ToolingScope project)))

(begin-foreign
  (c-define (tooling-project root) (int64) int64
    "marlin_tooling_project" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/tooling#marlin-tooling-project)))
