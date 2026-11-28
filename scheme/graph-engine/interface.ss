;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; -*- Gerbil -*-
;;; Public Marlin GraphEngine module composing `agentic-ai-lab`'s pure
;;; `GraphEngine` library with Marlin's Workbench UI, Rust continuation ABI
;;; projection, and opt-in `+sqlite-turso` substrate feature.

(import (only-in :clan/poo/object .def .get .o)
        (rename-in (only-in :tao3k/agentic-ai-lab/modules/graph-engine/interface GraphEngine)
                   (GraphEngine LabGraphEngine))
        (only-in "./config.ss" make-graph-engine-core)
        (only-in "./objects.ss" EnableMarlinGraphEngineSqliteTurso)
        (only-in "./types.ss" MarlinGraphEngineSqliteTursoContract))

(export GraphEngine)

(.def GraphEngine
  identity: 'graph-engine
  core: (make-graph-engine-core (.get LabGraphEngine core))
  features: (.o sqlite-turso: (.o refine: EnableMarlinGraphEngineSqliteTurso
                                  contract: MarlinGraphEngineSqliteTursoContract)))
