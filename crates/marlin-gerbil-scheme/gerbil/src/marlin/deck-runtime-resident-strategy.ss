;;; -*- Gerbil -*-
;;; Boundary: resident strategy procedure bridge for Rust-owned typed receipts.

package: marlin

(export marlin-deck-runtime-resident-strategy-execute)

;;; Boundary: Rust encodes resident requests as record-shaped Scheme data;
;;; this helper is the only field lookup adapter in the strategy bridge.
;; marlin-resident-strategy-record-ref
;;   : (-> SchemeDatum String SchemeDatum SchemeDatum)
;;   | doc m%
;;       Read one string-keyed field from a `(record ...)` datum and return a
;;       fallback when the request omits it.
;;
;;       # Examples
;;
;;       ```scheme
;;       (marlin-resident-strategy-record-ref
;;        '(record ("request_id" "r1")) "request_id" "unknown")
;;       ;; => "r1"
;;       ```
;;     %
(def (marlin-resident-strategy-record-ref record key default-value)
  (let select-field ((fields (if (and (pair? record) (eq? (car record) 'record))
                               (cdr record)
                               '())))
    (if (null? fields)
      default-value
      (let (entry (car fields))
        (if (and (pair? entry)
                 (equal? (car entry) key)
                 (pair? (cdr entry)))
          (cadr entry)
          (select-field (cdr fields)))))))

;;; Boundary: Resident strategy execution returns a typed record datum for the
;;; Rust bridge; policy decisions stay in Scheme, transport stays in Rust.
;; marlin-deck-runtime-resident-strategy-execute
;;   : (-> SchemeDatum SchemeDatum)
;;   | doc m%
;;       Project a resident runtime request into the response record expected
;;       by the Rust session loop.
;;
;;       # Examples
;;
;;       ```scheme
;;       (marlin-deck-runtime-resident-strategy-execute
;;        '(record ("request_id" "r1") ("lane_id" "main")))
;;       ;; => '(record ...)
;;       ```
;;     %
(def (marlin-deck-runtime-resident-strategy-execute request)
  (let* ((request-id
          (marlin-resident-strategy-record-ref request "request_id" "unknown-request"))
         (lane-id
          (marlin-resident-strategy-record-ref request "lane_id" "unknown-lane"))
         (event-kind
         (marlin-resident-strategy-record-ref request "event_kind" "unknown-event"))
         (session-id
          (marlin-resident-strategy-record-ref request "session_id" 'null))
         (policy-epoch
          (marlin-resident-strategy-record-ref request "policy_epoch" 'null))
         (payload
          (marlin-resident-strategy-record-ref request "payload" 'null))
         (derived-session-id
          (if (string? session-id)
            (string-append session-id ":gerbil-resident")
            "gerbil-resident-session")))
    (list 'record
          (list "status" "executed")
          (list "payload"
                (list 'record
                      (list "kind" "marlin.resident.strategy.procedure-response.v1")
                      (list "request_id" request-id)
                      (list "lane_id" lane-id)
                      (list "event_kind" event-kind)
                      (list "policy_epoch" policy-epoch)
                      (list "handled" #t)
                      (list "payload" payload)))
          (list "derived_session_id" derived-session-id))))
