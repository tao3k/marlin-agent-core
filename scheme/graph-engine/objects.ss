;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; -*- Gerbil -*-
;;; Boundary: POO slot bundle exposing Marlin's GraphEngine integration operations.

(import (only-in :clan/poo/object .def .o)
        (only-in :marlin-agent-core/interface LabDurableJournal)
        (only-in "./funcs.ss"
                 marlin-graph-engine-project-workbench-ui
                 marlin-graph-engine-dispatch->continuation
                 marlin-graph-engine-execute-turso-dispatch!
                 marlin-graph-engine-assemble-governed-context
                 marlin-graph-engine-project-dispatch-vector
                 marlin-graph-engine-reconcile-receipt-vector
                 marlin-graph-engine-project-workbench-tabs-vector
                 marlin-graph-engine-project-governed-context-vector
                 marlin-graph-engine-step-driver-vector
                 marlin-graph-engine-step-fleet-vector
                 marlin-graph-engine-fleet-checkpoint-vector
                 marlin-graph-engine-fleet-restore-vector)
        (only-in "./fleet-controller.ss"
                 make-marlin-fleet-workbench-controller
                 marlin-fleet-workbench-controller-apply-action
                 marlin-fleet-workbench-controller-project
                 marlin-fleet-workbench-controller-audit-capsule
                 marlin-fleet-workbench-controller-open-vector
                 marlin-fleet-workbench-controller-dispatch-vector))

(export GraphEngineMarlinOperations
        EnableMarlinGraphEngineSqliteTurso)

(.def GraphEngineMarlinOperations
  project-workbench-ui: marlin-graph-engine-project-workbench-ui
  dispatch->continuation: marlin-graph-engine-dispatch->continuation
  assemble-governed-context: marlin-graph-engine-assemble-governed-context
  project-dispatch-vector: marlin-graph-engine-project-dispatch-vector
  reconcile-receipt-vector: marlin-graph-engine-reconcile-receipt-vector
  project-workbench-tabs-vector: marlin-graph-engine-project-workbench-tabs-vector
  project-governed-context-vector: marlin-graph-engine-project-governed-context-vector
  step-driver-vector: marlin-graph-engine-step-driver-vector
  step-fleet-vector: marlin-graph-engine-step-fleet-vector
  fleet-checkpoint-vector: marlin-graph-engine-fleet-checkpoint-vector
  fleet-restore-vector: marlin-graph-engine-fleet-restore-vector
  make-fleet-workbench-controller: make-marlin-fleet-workbench-controller
  fleet-workbench-controller-apply-action: marlin-fleet-workbench-controller-apply-action
  fleet-workbench-controller-project: marlin-fleet-workbench-controller-project
  fleet-workbench-controller-audit-capsule: marlin-fleet-workbench-controller-audit-capsule
  fleet-workbench-controller-open-vector: marlin-fleet-workbench-controller-open-vector
  fleet-workbench-controller-dispatch-vector: marlin-fleet-workbench-controller-dispatch-vector)

(def (EnableMarlinGraphEngineSqliteTurso inherited)
  (.o (:: self inherited)
      execute-turso-dispatch!: marlin-graph-engine-execute-turso-dispatch!
      sqlite-turso: LabDurableJournal))
