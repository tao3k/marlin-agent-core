;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; -*- Gerbil -*-

(import (only-in :clan/poo/object .get .o)
        (only-in "./objects.ss" GraphEngineMarlinOperations))

(export make-graph-engine-core)

(def (make-graph-engine-core lab-core)
  (.o (:: @ lab-core)
      exports: (.o (:: @ [GraphEngineMarlinOperations (.get lab-core exports)]))))
