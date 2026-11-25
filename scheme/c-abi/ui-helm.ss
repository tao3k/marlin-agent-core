;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Native C-ABI entrypoint translating live Modern Helm prompt strings directly
;;; into rooted `search-engine` Playbook vectors for Rust ASP execution.
(import (only-in :marlin-agent-core/c-abi/roots marlin-rooted-call)
        (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/ui-helm/config UiHelmScope))
(export marlin-ui-helm-compile-plan)

(def (marlin-ui-helm-compile-plan input)
  (let ((prompt (if (string? input) input "@lang:rust marlin_search_engine")))
    ((.get UiHelmScope compile-plan-vector) prompt)))

(begin-foreign
  (c-define (ui-helm-compile-plan root) (int64) int64
    "marlin_ui_helm_compile_plan" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/ui-helm#marlin-ui-helm-compile-plan)))
