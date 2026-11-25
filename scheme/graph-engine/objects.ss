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
                 marlin-graph-engine-project-dispatch-vector
                 marlin-graph-engine-reconcile-receipt-vector
                 marlin-graph-engine-project-workbench-tabs-vector))

(export GraphEngineMarlinOperations
        EnableMarlinGraphEngineSqliteTurso)

(.def GraphEngineMarlinOperations
  project-workbench-ui: marlin-graph-engine-project-workbench-ui
  dispatch->continuation: marlin-graph-engine-dispatch->continuation
  project-dispatch-vector: marlin-graph-engine-project-dispatch-vector
  reconcile-receipt-vector: marlin-graph-engine-reconcile-receipt-vector
  project-workbench-tabs-vector: marlin-graph-engine-project-workbench-tabs-vector)

(def (EnableMarlinGraphEngineSqliteTurso inherited)
  (.o (:: self inherited)
      execute-turso-dispatch!: marlin-graph-engine-execute-turso-dispatch!
      sqlite-turso: LabDurableJournal))
