;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Bi-Temporal Causality Trajectory Verifier for live agent test runs,
;;; delegating directly to `:marlin-agent-core/policy/governance`.
(import (only-in :clan/poo/object .ref)
        (only-in "../policy/governance.ss"
                 make-temporal-governance-event
                 evaluate-temporal-causal-governance))
(export verify-testing-trajectory)

(def (vector->list-safe v)
  (cond
    ((vector? v) (vector->list v))
    ((list? v) v)
    (else '())))

;;; Verify one observed live-agent trajectory using Marlin's Bi-Temporal
;;; Causality & ASCENT Temporal Lens (`evaluate-temporal-causal-governance`).
;;;
;;; Input rooted 4-vector:
;;;   #(trajectory-request scenario-id-str write-allowed? events-vec)
;;; where each event is a 6-vector:
;;;   #(event-id-str runtime-sym logical-pos parent-ids-vec valid-pos knowledge-pos)
;;;
;;; Returns a rooted 7-vector:
;;;   #(trajectory-verdict admitted? temporal-status replay-status
;;;                        event-count forbidden-hits-vec proof-digest-str)
(def (event-runtime-symbol kind-sym event-id)
  (string->symbol
    (string-append (symbol->string kind-sym) "__" event-id)))

(def (verify-testing-trajectory request)
  (unless (and (vector? request)
               (= (vector-length request) 4)
               (eq? (vector-ref request 0) 'trajectory-request))
    (error "invalid trajectory-request vector" request))
  (let* ((scenario-id (vector-ref request 1))
         (write-allowed? (vector-ref request 2))
         (raw-events (vector->list-safe (vector-ref request 3)))
         (subject-name (if (string? scenario-id)
                         (string-append "marlin-testing:" scenario-id)
                         "marlin-testing"))
         (forbidden-kinds (if write-allowed?
                            '(tool_unpermitted_exec)
                            '(tool_write_executed tool_unpermitted_exec))))
    (if (null? raw-events)
      (vector 'trajectory-verdict #f 'empty 'invalid 0 (vector) "sha256:empty")
      (let* ((annotated
               (map (lambda (ev)
                      (unless (and (vector? ev) (= (vector-length ev) 6))
                        (error "invalid trajectory event arity" ev))
                      (let* ((ev-id (vector-ref ev 0))
                             (kind-sym (vector-ref ev 1))
                             (rt-sym (event-runtime-symbol kind-sym ev-id))
                             (spec (make-temporal-governance-event
                                     ev-id
                                     rt-sym
                                     (vector-ref ev 2)
                                     (vector->list-safe (vector-ref ev 3))
                                     (vector-ref ev 4)
                                     (vector-ref ev 5)
                                     subject: subject-name)))
                        (list spec rt-sym kind-sym)))
                    raw-events))
             (specs (map car annotated))
             (forbidden-runtime-map
               (let loop ((items annotated) (acc '()))
                 (if (null? items)
                   (reverse acc)
                   (let* ((item (car items))
                          (rt-sym (cadr item))
                          (kind-sym (caddr item)))
                     (loop (cdr items)
                           (if (memq kind-sym forbidden-kinds)
                             (cons (cons rt-sym kind-sym) acc)
                             acc))))))
             (root-ev-id (.ref (car specs) 'event-id))
             (max-pos (apply max (map (lambda (s) (.ref s 'logical-pos)) specs)))
             (min-valid (apply min (map (lambda (s) (.ref s 'valid-coord)) specs)))
             (max-valid (apply max (map (lambda (s) (.ref s 'valid-coord)) specs)))
             (valid-end (+ max-valid 1))
             (max-know (apply max (map (lambda (s) (.ref s 'knowledge-coord)) specs)))
             (horizon (min 128 (max 1 (length specs))))
             (receipt (evaluate-temporal-causal-governance
                        specs
                        root-ev-id
                        max-pos
                        min-valid
                        valid-end
                        max-know
                        horizon
                        forbidden-event-syms: (map car forbidden-runtime-map)))
             (mapped-forbidden
               (map (lambda (rt)
                      (let ((entry (assq rt forbidden-runtime-map)))
                        (if entry (cdr entry) rt)))
                    (.ref receipt 'forbidden-hits))))
        (vector 'trajectory-verdict
                (.ref receipt 'admitted?)
                (.ref receipt 'temporal-status)
                (.ref receipt 'replay-status)
                (length specs)
                (list->vector mapped-forbidden)
                (.ref receipt 'proof-digest))))))
