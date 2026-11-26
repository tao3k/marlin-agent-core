;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; -*- Gerbil -*-
;;; Minimal durable journal POO facade for GraphEngine, avoiding the kitchen-sink
;;; `:marlin-agent-core/interface` import (which pulls in orgize, gitops, and
;;; module-authoring catalogs into the Linked-AOT closure).

(import (only-in :clan/poo/object .o)
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/runtime/durable-journal
                 journal-open journal-close! journal-receive! journal-begin! journal-commit!
                 journal-put! journal-get journal-rows journal-verify! journal-pack journal-hash))

(export MarlinDurableJournal)

(def MarlinDurableJournal
  (.o open: journal-open
      close!: journal-close!
      receive!: journal-receive!
      begin!: journal-begin!
      commit!: journal-commit!
      put!: journal-put!
      get: journal-get
      rows: journal-rows
      verify!: journal-verify!
      pack: journal-pack
      hash: journal-hash))
