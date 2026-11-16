;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Use Lab's causal exchange and preserve the full ASCENT answer.

(import (only-in :marlin-agent-core/interface LabTemporal)
        (only-in :clan/poo/object .get .call))

(export context-reason-causally context-replay-causal-answer)

(def (context-reason-causally request)
  (.call LabTemporal ascent-exchange-solve (.get request exchange) (.get request root)))

(def (context-replay-causal-answer request)
  (.call LabTemporal ascent-exchange-verify (.get request exchange) (.get request root) (.get request answer)))
