;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; The selected Context publication owns source parsing. These procedures
;;; exchange Scheme values through the SDK's strong-root boundary.
(import (only-in :clan/poo/object .get .o)
        (only-in :std/list/list length<=n?))
(export marlin-context-org-source-open marlin-context-org-source-project
        marlin-context-org-source-fields marlin-context-org-contract-open
        marlin-context-org-contract-project)

;;; Reject oversized or improper lists before allocating a native projection or
;;; per-call lookup index. The parser has already produced its source graph;
;;; this bound does not claim to limit the parser's own working memory.
(def (bounded-records! values limit)
  (unless (and (length<=n? values limit) (list? values))
    (error "Context Org record limit exceeded or improper record list")))

(def (marlin-context-org-contract-open input)
  (unless (and (vector? input) (= (vector-length input) 4))
    (error "invalid Context Org contract source request"))
  (let* ((selection (vector-ref input 0))
         (identity-value (vector-ref input 1))
         (generation-value (vector-ref input 2))
         (source-value (vector-ref input 3))
         (context (.get selection Marlin context)))
    (let (source ((.get context org-contract-source)
                  (.o identity: identity-value generation: generation-value source: source-value)))
      (bounded-records! (.get source nodes) 32)
      source)))

(def (marlin-context-org-contract-project input)
  (bounded-records! (.get input nodes) 32)
  (vector (.get input identity) (.get input generation) (.get input source)
          (.get input source-digest) (.get input semantic-digest)
          (.get input contract-identity) (.get input contract-revision)
          (.get input session-identity) (.get input parser-identity)
          (list->vector
           (map (lambda (node)
                  (vector (.get node identity) (.get node propertyId)
                          (.get node elementKind) (.get node byteStart)))
                (.get input nodes)))))

(def (marlin-context-org-source-open input)
  (unless (and (vector? input) (= (vector-length input) 2)
               (string? (vector-ref input 1)))
    (error "invalid Context Org source request"))
  (let* ((selection (vector-ref input 0))
         (source-value (vector-ref input 1))
         (context (.get selection Marlin context)))
    (let (source ((.get context org-source-graph) source-value))
      (bounded-records! (.get (.get source graph) records) 4096)
      source)))

(def (marlin-context-org-source-project input)
  (let* ((source-value (.get input source))
         (records (.get (.get input graph) records)))
    (bounded-records! records 4096)
    (vector source-value
            (list->vector
             (map (lambda (record)
                    (vector (.get record id) (.get record parent)
                            (.get record kind) (.get record start) (.get record end)))
                  records)))))

;;; A bounded batch retains the graph's own field-of semantics, including
;;; computed headline properties. No field text is re-parsed by the host.
(def (marlin-context-org-source-fields input)
  (unless (and (vector? input) (= (vector-length input) 2)
               (vector? (vector-ref input 1)) (<= (vector-length (vector-ref input 1)) 64))
    (error "invalid Org field batch"))
  (let* ((graph (.get (vector-ref input 0) graph))
         (records (.get graph records))
         (field-of (.get graph field-of))
         (index (make-hash-table))
         (total-items 0)
         (requests (vector-ref input 1)))
    (bounded-records! records 4096)
    (for-each (lambda (record) (hash-put! index (.get record id) record)) records)
    (def (field-value value)
      (cond ((not value) #f)
            ((string? value) value)
            ((or (null? value) (pair? value))
             ;; Validate and charge once, stopping even on a cyclic list.
             (let loop ((remaining value) (count 0))
               (cond ((null? remaining)
                      (set! total-items (+ total-items count))
                      (when (> total-items 8192) (error "too many Org field values"))
                      (list->vector value))
                     ((or (>= count 4096) (not (pair? remaining)) (not (string? (car remaining))))
                      (error "invalid Org field list"))
                     (else (loop (cdr remaining) (+ count 1))))))
            (else (error "unsupported Org field value"))))
    (vector-map
     (lambda (request)
       (unless (and (vector? request) (= (vector-length request) 2)
                    (exact-integer? (vector-ref request 0)) (<= 0 (vector-ref request 0))
                    (string? (vector-ref request 1)) (< 0 (string-length (vector-ref request 1)) 129))
         (error "invalid Org field request"))
       (let (record (hash-get index (vector-ref request 0)))
         (unless record (error "Org element not found"))
         (field-value (field-of record (vector-ref request 1)))))
     requests)))
