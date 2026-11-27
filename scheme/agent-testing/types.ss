;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Typed POO contract for Marlin's declarative Agent Testing module.
(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/type Exactly Fun Record Type.))
(export AgentTestingScopeContract)

(def NativeSuiteSelector
  (.cc Type. '.element? (lambda (x) (or (void? x) (not x) (symbol? x) (string? x)))))
(def NativeTestingSuite (.cc Type. '.element? vector?))
(def NativeTrajectoryRequest (.cc Type. '.element? vector?))
(def NativeTrajectoryVerdict (.cc Type. '.element? vector?))

(def AgentTestingScopeContract
  (.cc (Record identity: [(Exactly 'marlin/agent-testing)]
               project-suite: [(Fun NativeTestingSuite <- NativeSuiteSelector)]
               verify-trajectory: [(Fun NativeTrajectoryVerdict <- NativeTrajectoryRequest)])
       identity: 'marlin/agent-testing))
