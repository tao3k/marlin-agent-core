;;; -*- Gerbil -*-
;;; Engineering note: Slot merge receipts model policy algebra, not generic list
;;; utilities. Keep the vector normalization here because Rust consumes stable
;;; receipt order while Scheme owns the merge semantics and conflict labels.
package: config-interface/modules

(import (only-in :clan/poo/object .get .o)
        (only-in :poo-flow/src/module-system/facade poo-flow-scheme-owner)
        :config-interface/modules/policy-pack-support)

(export marlin-policy-slot-merge-receipt-kind
        marlin-policy-slot-merge-algebra-demo-receipt-kind
        marlinPolicySlotMergeReceipt
        marlinPolicySlotMergeUnion
        marlinPolicySlotMergeIntersection
        marlinPolicySlotMergeMin
        marlinPolicySlotMergeOrderedAppend
        marlinPolicySlotMergeConflictError
        marlinPolicySlotMergeAuditReceipt
        marlinPolicySlotMergeAuditReceipts
        marlinPolicySlotMergeForcedSlots
        marlinPolicySlotMergeAlgebraDemoReceipt)

;;; Boundary: Slot merge receipts prove POO policy algebra as Scheme values.
;; : (-> MarlinInput MarlinResult)
(def marlin-policy-slot-merge-receipt-kind
  "marlin.config-interface.policy-pack.slot-merge-receipt.v1")

(def marlin-policy-slot-merge-algebra-demo-receipt-kind
  "marlin.config-interface.policy-pack.slot-merge-algebra-demo-receipt.v1")

;; MarlinResult <- MarlinInput
(def (marlin-policy-slot-merge-values->list values)
  (cond
   ((vector? values) (vector->list values))
   ((list? values) values)
   (else (list values))))

;; MarlinResult <- MarlinInput
(def (marlin-policy-slot-merge-values->vector values)
  (cond
   ((vector? values) values)
   ((list? values) (list->vector values))
   (else (vector values))))

;; MarlinResult <- MarlinInput
(def (marlin-policy-slot-merge-list->unique-vector values)
  (list->vector
   (foldl (lambda (value result)
            (if (member value result)
              result
              (append result (list value))))
          '()
          values)))

;; MarlinResult <- MarlinInput
(def (marlinPolicySlotMergeReceipt slot-id-value
                                   slot-value
                                   merge-value
                                   status-value
                                   input-values
                                   result-value
                                   conflict-reasons-value)
  (marlin-policy-object<-alist
   (list
    (cons 'kind marlin-policy-slot-merge-receipt-kind)
    (cons 'slot_id slot-id-value)
    (cons 'slot slot-value)
    (cons 'merge merge-value)
    (cons 'status status-value)
    (cons 'inputs input-values)
    (cons 'result result-value)
    (cons 'conflict-reasons
          (marlin-policy-slot-merge-values->vector conflict-reasons-value))
    (cons 'owner poo-flow-scheme-owner)
    (cons 'scheme-boundary "scheme-types-to-rust-types")
    (cons 'serialization-boundary "rust-owned-cli-trace-cross-process")
    (cons 'rust-handler-manufactured #f))))

;; MarlinResult <- MarlinInput
(def (marlinPolicySlotMergeUnion slot-id-value
                                 slot-value
                                 left-values
                                 right-values)
  (let ((left-vector
         (marlin-policy-slot-merge-values->vector left-values))
        (right-vector
         (marlin-policy-slot-merge-values->vector right-values)))
    (marlinPolicySlotMergeReceipt
     slot-id-value
     slot-value
     "union"
     "merged"
     (vector left-vector right-vector)
     (marlin-policy-slot-merge-list->unique-vector
      (append (vector->list left-vector)
              (vector->list right-vector)))
     (vector))))

;; MarlinResult <- MarlinInput
(def (marlinPolicySlotMergeIntersection slot-id-value
                                        slot-value
                                        left-values
                                        right-values)
  (let* ((left-vector
          (marlin-policy-slot-merge-values->vector left-values))
         (right-vector
          (marlin-policy-slot-merge-values->vector right-values))
         (right-list (vector->list right-vector)))
    (marlinPolicySlotMergeReceipt
     slot-id-value
     slot-value
     "intersection"
     "merged"
     (vector left-vector right-vector)
     (marlin-policy-slot-merge-list->unique-vector
      (filter (lambda (value)
                (member value right-list))
              (vector->list left-vector)))
     (vector))))

;; MarlinResult <- MarlinInput
(def (marlinPolicySlotMergeMin slot-id-value
                               slot-value
                               left-value
                               right-value)
  (marlinPolicySlotMergeReceipt
   slot-id-value
   slot-value
   "min"
   "merged"
   (vector left-value right-value)
   (if (< left-value right-value) left-value right-value)
   (vector)))

;; MarlinResult <- MarlinInput
(def (marlinPolicySlotMergeOrderedAppend slot-id-value
                                         slot-value
                                         left-values
                                         right-values)
  (let ((left-vector
         (marlin-policy-slot-merge-values->vector left-values))
        (right-vector
         (marlin-policy-slot-merge-values->vector right-values)))
    (marlinPolicySlotMergeReceipt
     slot-id-value
     slot-value
     "ordered_append"
     "merged"
     (vector left-vector right-vector)
     (list->vector
      (append (vector->list left-vector)
              (vector->list right-vector)))
     (vector))))

;; MarlinResult <- MarlinInput
(def (marlinPolicySlotMergeConflictError slot-id-value
                                         slot-value
                                         left-value
                                         right-value)
  (if (equal? left-value right-value)
    (marlinPolicySlotMergeReceipt
     slot-id-value
     slot-value
     "conflict_error"
     "merged"
     (vector left-value right-value)
     left-value
     (vector))
    (marlinPolicySlotMergeReceipt
     slot-id-value
     slot-value
     "conflict_error"
     "conflict"
     (vector left-value right-value)
     #f
     (vector "exclusive-resource-conflict"))))

;; MarlinResult <- MarlinInput
(def (marlin-policy-slot-merge-audit-status status-value)
  (cond
   ((string=? status-value "merged") "applied")
   ((string=? status-value "applied") "applied")
   ((string=? status-value "conflict") "conflict")
   (else status-value)))

;;; Boundary: Audit packs keep Rust's compact SlotMergeReceipt IR stable.
;; MarlinResult <- MarlinInput
(def (marlinPolicySlotMergeAuditReceipt slot-merge-receipt)
  (.o slot_id: (.get slot-merge-receipt slot_id)
      merge: (.get slot-merge-receipt merge)
      status:
      (marlin-policy-slot-merge-audit-status
       (.get slot-merge-receipt status))))

;; MarlinResult <- MarlinInput
(def (marlinPolicySlotMergeAuditReceipts slot-merge-receipts)
  (marlin-vector-map marlinPolicySlotMergeAuditReceipt slot-merge-receipts))

;; MarlinResult <- MarlinInput
(def (marlinPolicySlotMergeForcedSlots slot-merge-receipts hotness-value)
  (marlin-vector-map
   (lambda (slot-merge-receipt)
     (.o slot_id: (.get slot-merge-receipt slot_id)
         hotness: hotness-value))
   slot-merge-receipts))

;;; Boundary: Public algebra demo is the first POO policy combination receipt.
;; MarlinResult <- MarlinInput
(def (marlinPolicySlotMergeAlgebraDemoReceipt)
  (let* ((capability-receipt
          (marlinPolicySlotMergeIntersection
           100
           "capability"
           (vector "+read" "+write" "+tool")
           (vector "+read" "+tool" "+verify")))
         (denylist-receipt
          (marlinPolicySlotMergeUnion
           101
           "denylist"
           (vector "secrets/.env")
           (vector "target/" "secrets/.env")))
         (human-gates-receipt
          (marlinPolicySlotMergeUnion
           102
           "human_gates"
           (vector "security-review")
           (vector "cost-review")))
         (budget-receipt
          (marlinPolicySlotMergeMin
           103
           "budget.max_attempts"
           3
           2))
         (route-rules-receipt
          (marlinPolicySlotMergeOrderedAppend
           104
           "route_rules"
           (vector "model" "tool")
           (vector "verify" "stop")))
         (exclusive-resource-receipt
          (marlinPolicySlotMergeConflictError
           105
           "exclusive_resource"
           "workspace-write"
           "repo-admin"))
         (observability-receipt
          (marlinPolicySlotMergeUnion
           106
           "observability"
           (vector "runtime.tool")
           (vector "harness.execution" "runtime.tool"))))
    (.o kind: marlin-policy-slot-merge-algebra-demo-receipt-kind
        profile-id: "policy-merge-algebra-demo"
        owner: poo-flow-scheme-owner
        receipts:
        (vector capability-receipt
                denylist-receipt
                human-gates-receipt
                budget-receipt
                route-rules-receipt
                exclusive-resource-receipt
                observability-receipt)
        receipt-count: 7
        required-laws:
        (vector "capability=intersection"
                "denylist=union"
                "human_gates=union"
                "budget.max_attempts=min"
                "route_rules=ordered_append"
                "exclusive_resource=conflict_error"
                "observability=union")
        scheme-boundary: "scheme-types-to-rust-types"
        serialization-boundary: "rust-owned-cli-trace-cross-process"
        rust-handler-manufactured: #f)))
