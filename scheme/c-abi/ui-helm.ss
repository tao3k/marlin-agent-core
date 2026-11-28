;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Native C-ABI entrypoint translating live Modern Helm prompt strings directly
;;; into rooted `search-engine` Playbook vectors for Rust ASP execution.
(import (only-in :marlin-agent-core/c-abi/roots marlin-rooted-call)
        (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/ui-helm/config UiHelmScope)
        (only-in :marlin-agent-core/ui/helm-workbench
                 make-marlin-helm-workbench-package))
(export marlin-ui-helm-compile-plan
        marlin-ui-helm-index-config
        marlin-ui-helm-window-policy
        make-marlin-helm-workbench-package)

(def (marlin-ui-helm-compile-plan input)
  (let ((prompt (if (string? input) input "@lang:rust marlin_search_engine")))
    ((.get UiHelmScope compile-plan-vector) prompt)))

(def (marlin-ui-helm-index-config _input)
  ((.get UiHelmScope index-config-vector)))

(def (marlin-ui-helm-window-policy _input)
  ((.get UiHelmScope window-policy-vector)))

(begin-foreign
  (c-define (ui-helm-compile-plan root) (int64) int64
    "marlin_ui_helm_compile_plan" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/ui-helm#marlin-ui-helm-compile-plan))
  (c-define (ui-helm-index-config root) (int64) int64
    "marlin_ui_helm_index_config" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/ui-helm#marlin-ui-helm-index-config))
  (c-define (ui-helm-window-policy root) (int64) int64
    "marlin_ui_helm_window_policy" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/ui-helm#marlin-ui-helm-window-policy)))
