;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/mop .new/checked)
        "./objects.ss"
        "./funcs.ss"
        "./types.ss")
(export UiModesScope)

(def UiModesScope
  (.new/checked UiModesScopeContract
    identity: 'marlin/ui-modes
    major-modes: DefaultMajorModes
    minor-modes: DefaultMinorModes
    inner-modes: DefaultInnerModes
    polymodes: DefaultPolymodes
    buffer-mode-rules: DefaultBufferModeRules
    parent-chain: marlin-modes-parent-chain
    resolve-buffer-mode: marlin-modes-resolve-buffer-mode
    resolve-polymode-inner: marlin-modes-resolve-polymode-inner
    make-buffer-state: marlin-modes-make-buffer-state
    effective-stack: marlin-modes-effective-stack
    lookup-key: marlin-modes-lookup-key
    switch-span: marlin-modes-switch-span
    toggle-minor-mode: marlin-modes-toggle-minor-mode
    project-grpc-registry: marlin-modes-project-grpc-registry))
