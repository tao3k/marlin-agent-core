;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Checked scope export for Marlin's Modern Helm (`scheme/ui-helm`).
(import (only-in :clan/poo/mop .new/checked)
        "./funcs.ss"
        "./types.ss")
(export UiHelmScope)

(def UiHelmScope
  (.new/checked HelmScopeContract
    identity: 'marlin/ui-helm
    default-sources: (default-helm-sources)
    default-actions: (default-helm-actions)
    open-session: (lambda () (helm-open-session))
    parse-input: helm-parse-input
    update-input: helm-update-input
    compile-plan-vector: (lambda (raw-text) (helm-compile-plan-vector raw-text))
    apply-results: helm-apply-results
    move-selection: helm-move-selection
    toggle-mark: helm-toggle-mark
    next-source: helm-next-source
    toggle-preview: helm-toggle-preview
    toggle-actions: helm-toggle-actions
    dispatch-action: helm-dispatch-action
    render-lines: helm-render-lines
    project-ui-elements: helm-project-ui-elements
    index-config-vector: (lambda () (helm-project-index-config))
    window-policy-vector: (lambda () (helm-project-window-policy))))
