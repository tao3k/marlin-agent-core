;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Exercise the Context publication against real Lab cuts and ASCENT receipts.

(import (only-in :clan/poo/object .get .o)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports)
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/objects
                 agentic-ai-lab-causal-event agentic-ai-lab-temporal-observation)
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/funs
                 agentic-ai-lab-causal-event-graph agentic-ai-lab-causal-cut)
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/time/objects
                 agentic-ai-lab-temporal-instant)
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/ascent-exchange
                 agentic-ai-lab-ascent-exchange agentic-ai-lab-ascent-event-binding)
        (only-in :gerbil-ascent/temporal/lens temporal-status temporal-rows temporal-receipt)
        (only-in :std/test check))

(def (event id position parents)
  (agentic-ai-lab-causal-event id "subject" 'observed
    (agentic-ai-lab-temporal-observation id 'logical-version position "ledger")
    id parents 'observed #t))

(def (instant id domain position)
  (agentic-ai-lab-temporal-instant id domain position "owner" 'observed))

(def graph
  (agentic-ai-lab-causal-event-graph "subject"
    (list (event "A" 0 '()) (event "B" 1 '("A")))))
(def event-cut (agentic-ai-lab-causal-cut graph 1))

(def (make-exchange knowledge-id)
  (agentic-ai-lab-ascent-exchange graph event-cut
    (list
     (agentic-ai-lab-ascent-event-binding "A" 'a
       (instant "a-valid" "valid" 0) (instant "a-known" "knowledge" 0))
     (agentic-ai-lab-ascent-event-binding "B" 'b
       (instant "b-valid" "valid" 1) (instant knowledge-id "knowledge" 1)))
    'owner-source 0 'owner-clock "valid" "knowledge" 'owner-cut 0 5 4 3 #t))

(let* ((context (.get marlin-agent-core.init.exports Marlin context))
       (original (make-exchange "b-known"))
       (answer-value ((.get context reason-causally) (.o exchange: original root: "A")))
       (replay (.get context replay-causal-answer)))
  (check (temporal-status answer-value) => 'complete)
  (check (temporal-rows answer-value) => '((a b)))
  (check (and (temporal-receipt answer-value) #t) => #t)
  (check (replay (.o exchange: original root: "A" answer: answer-value)) => 'valid)
  (check (replay (.o exchange: (make-exchange "b-known-changed") root: "A" answer: answer-value)) => 'invalid))
