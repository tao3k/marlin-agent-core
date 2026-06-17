;;; -*- Gerbil -*-
;;; Boundary: Policy object furniture and deterministic object surgery.

package: modules

(import (only-in :clan/poo/object .get .has? .o object?)
        :modules/kinds)

(export marlinPolicyObject
        marlin-policy-object?
        marlin-policy-object-id
        marlin-policy-object-family
        marlin-policy-object-ids
        marlin-policy-object-families
        marlin-policy-object-disabled-ids
        marlin-add-object
        marlin-remove-object
        marlin-disable-object
        marlin-replace-object
        marlin-policy-object-operation?
        marlin-policy-pack-apply-operations
        marlin-policy-surgery-conflict-reason-count
        marlin-policy-disabled-object-count)

;;; Boundary: Policy objects wrap existing POO values without changing handlers.
;; MarlinResult <- MarlinInput
(def (marlinPolicyObject
      family-value
      object-id-value
      object-value
      . maybe-metadata-value)
  (let (metadata-value
        (if (null? maybe-metadata-value)
          '()
          (car maybe-metadata-value)))
    (.o (:: @ (list object-value))
        id: object-id-value
        policy-object-kind: marlin-policy-object-kind
        policy-object-family: family-value
        policy-object-id: object-id-value
        policy-object-disabled: #f
        policy-object-payload: object-value
        policy-object-metadata: metadata-value
        policy-object-owner: "gerbil-poo"
        rust-handler-manufactured: #f)))

;;; Boundary: Predicate identifies managed policy objects by sidecar kind.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object? value)
  (and (object? value)
       (.has? value policy-object-kind)
       (string=? (.get value policy-object-kind)
                 marlin-policy-object-kind)))

;;; Boundary: Object ids are stable within a policy object family.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-id value)
  (.get value policy-object-id))

;;; Boundary: Families group policy objects without Rust knowing merge rules.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-family value)
  (.get value policy-object-family))

;;; Boundary: Object id inventory stays stable across payload shapes.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-ids object-values)
  (map marlin-policy-object-id
       (filter marlin-policy-object? object-values)))

;;; Boundary: Family inventory is ordered by first appearance in the pack.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-families object-values)
  (foldl (lambda (object-value family-values)
           (if (marlin-policy-object? object-value)
             (let (family-value
                   (marlin-policy-object-family object-value))
               (if (member family-value family-values)
                 family-values
                 (append family-values (list family-value))))
             family-values))
         '()
         object-values))

;;; Boundary: Disabled object ids make surgery visible without dropping payloads.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-disabled-ids object-values)
  (map marlin-policy-object-id
       (filter (lambda (object-value)
                 (and (marlin-policy-object? object-value)
                      (.get object-value policy-object-disabled)))
               object-values)))

;;; Boundary: Disabled objects remain visible in presentations and receipts.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-disable object-value reason-value)
  (let (payload-value
        (.get object-value policy-object-payload))
    (.o (:: @ (list payload-value))
        id: (marlin-policy-object-id object-value)
        policy-object-kind: marlin-policy-object-kind
        policy-object-family: (marlin-policy-object-family object-value)
        policy-object-id: (marlin-policy-object-id object-value)
        policy-object-disabled: #t
        policy-object-disabled-reason: reason-value
        policy-object-payload: payload-value
        policy-object-metadata: (.get object-value policy-object-metadata)
        policy-object-owner: "gerbil-poo"
        rust-handler-manufactured: #f)))

;;; Boundary: Object operations carry intent plus receipts, not handler code.
;; MarlinResult <- MarlinInput
(def (marlinPolicyObjectOperation
      operation-value
      family-value
      target-id-value
      object-value
      replacement-value
      maybe-values)
  (let ((reason-value
         (if (null? maybe-values)
           "policy-object-surgery"
           (car maybe-values)))
        (metadata-value
         (if (or (null? maybe-values)
                 (null? (cdr maybe-values)))
           '()
           (cadr maybe-values))))
    (.o kind: marlin-policy-object-operation-kind
        operation: operation-value
        family: family-value
        target-id: target-id-value
        operation-object: object-value
        operation-replacement: replacement-value
        reason: reason-value
        metadata: metadata-value
        owner: "gerbil-poo"
        rust-handler-manufactured: #f)))

;;; Boundary: Add operations append a managed policy object to the prefab pack.
;; MarlinResult <- MarlinInput
(def (marlin-add-object object-value . maybe-values)
  (marlinPolicyObjectOperation
   "add"
   (marlin-policy-object-family object-value)
   (marlin-policy-object-id object-value)
   object-value
   #f
   maybe-values))

;;; Boundary: Remove operations delete matching furniture by family/id.
;; MarlinResult <- MarlinInput
(def (marlin-remove-object family-value target-id-value . maybe-values)
  (marlinPolicyObjectOperation
   "remove"
   family-value
   target-id-value
   #f
   #f
   maybe-values))

;;; Boundary: Disable operations keep the object visible but inactive.
;; MarlinResult <- MarlinInput
(def (marlin-disable-object family-value target-id-value . maybe-values)
  (marlinPolicyObjectOperation
   "disable"
   family-value
   target-id-value
   #f
   #f
   maybe-values))

;;; Boundary: Replace operations swap one family/id object for another POO object.
;; MarlinResult <- MarlinInput
(def (marlin-replace-object
      family-value
      target-id-value
      replacement-value
      . maybe-values)
  (marlinPolicyObjectOperation
   "replace"
   family-value
   target-id-value
   #f
   replacement-value
   maybe-values))

;;; Boundary: Operation detection is typed by kind, not list syntax.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-operation? value)
  (and (object? value)
       (.has? value kind)
       (string=? (.get value kind) marlin-policy-object-operation-kind)))

;;; Boundary: Object surgery matches only managed family/id pairs.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-matches? object-value family-value target-id-value)
  (and (marlin-policy-object? object-value)
       (equal? (marlin-policy-object-family object-value) family-value)
       (equal? (marlin-policy-object-id object-value) target-id-value)))

;;; Boundary: Receipt ids stay scalar for Rust/debug projections.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-id/default object-value default-value)
  (if (marlin-policy-object? object-value)
    (marlin-policy-object-id object-value)
    default-value))

;;; Boundary: Target lookup is shared by object surgery and conflict receipts.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-targets object-values family-value target-id-value)
  (filter (lambda (candidate)
            (marlin-policy-object-matches?
             candidate
             family-value
             target-id-value))
          object-values))

;;; Boundary: Disabled targets are conflicts, not silent surgery targets.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-targets-disabled? object-values)
  (> (length
      (filter (lambda (object-value)
                (and (marlin-policy-object? object-value)
                     (.get object-value policy-object-disabled)))
              object-values))
     0))

;;; Boundary: Conflict reasons stay deterministic scalar receipt values.
;; MarlinResult <- MarlinInput
(def (marlin-policy-conflict-append
      condition-value
      reason-value
      conflict-reasons-value)
  (if condition-value
    (append conflict-reasons-value (list reason-value))
    conflict-reasons-value))

;;; Boundary: Surgery receipts prove Scheme object composition decisions.
;; MarlinResult <- MarlinInput
(def (marlin-policy-object-surgery-receipt
      operation-value
      matched-count-value
      conflict-reasons-value)
  (.o kind: marlin-policy-object-surgery-receipt-kind
      operation: (.get operation-value operation)
      family: (.get operation-value family)
      target-id: (.get operation-value target-id)
      object-id:
      (marlin-policy-object-id/default
       (.get operation-value operation-object)
       #f)
      replacement-id:
      (marlin-policy-object-id/default
       (.get operation-value operation-replacement)
       #f)
      matched?: (> matched-count-value 0)
      matched-count: matched-count-value
      valid?: (null? conflict-reasons-value)
      conflict?: (not (null? conflict-reasons-value))
      conflict-reasons: conflict-reasons-value
      owner: "gerbil-poo"
      rust-handler-manufactured: #f))

;;; Boundary: A single operation transforms objects and returns one receipt.
;; MarlinResult <- MarlinInput
(def (marlin-policy-pack-apply-operation object-values operation-value)
  (let* ((operation-name (.get operation-value operation))
         (family-value (.get operation-value family))
         (target-id-value (.get operation-value target-id)))
    (cond
     ((string=? operation-name "add")
      (let* ((operation-object-value
              (.get operation-value operation-object))
             (duplicate-values
              (marlin-policy-object-targets
               object-values
               (marlin-policy-object-family operation-object-value)
               (marlin-policy-object-id operation-object-value)))
             (conflict-reasons-value
              (marlin-policy-conflict-append
               (pair? duplicate-values)
               "duplicate-object"
               '())))
        (.o policy-objects:
            (if (null? conflict-reasons-value)
              (append object-values (list operation-object-value))
              object-values)
            matched-count:
            (if (pair? duplicate-values)
              (length duplicate-values)
              1)
            conflict-reasons: conflict-reasons-value)))
     ((string=? operation-name "remove")
      (let* ((matched
              (marlin-policy-object-targets
               object-values
               family-value
               target-id-value))
             (conflict-reasons-value
              (marlin-policy-conflict-append
               (marlin-policy-object-targets-disabled? matched)
               "disabled-target"
               (marlin-policy-conflict-append
                (null? matched)
                "missing-target"
                '())))
            (kept
             (filter (lambda (candidate)
                       (not
                        (marlin-policy-object-matches?
                         candidate
                        family-value
                        target-id-value)))
                     object-values)))
        (.o policy-objects:
            (if (null? conflict-reasons-value)
              kept
              object-values)
            matched-count: (length matched)
            conflict-reasons: conflict-reasons-value)))
     ((string=? operation-name "disable")
      (let* ((matched
              (marlin-policy-object-targets
               object-values
               family-value
               target-id-value))
             (conflict-reasons-value
              (marlin-policy-conflict-append
               (marlin-policy-object-targets-disabled? matched)
               "disabled-target"
               (marlin-policy-conflict-append
                (null? matched)
                "missing-target"
                '()))))
        (.o policy-objects:
            (if (null? conflict-reasons-value)
              (map (lambda (candidate)
                     (if (marlin-policy-object-matches?
                          candidate
                          family-value
                          target-id-value)
                       (marlin-policy-object-disable
                        candidate
                        (.get operation-value reason))
                       candidate))
                   object-values)
              object-values)
            matched-count: (length matched)
            conflict-reasons: conflict-reasons-value)))
     ((string=? operation-name "replace")
      (let* ((matched
              (marlin-policy-object-targets
               object-values
               family-value
               target-id-value))
             (replacement-value
              (.get operation-value operation-replacement))
             (conflict-reasons-value
              (marlin-policy-conflict-append
               (not (marlin-policy-object? replacement-value))
               "invalid-replacement"
               (marlin-policy-conflict-append
                (marlin-policy-object-targets-disabled? matched)
                "disabled-target"
                (marlin-policy-conflict-append
                 (null? matched)
                 "missing-target"
                 '())))))
        (.o policy-objects:
            (if (null? conflict-reasons-value)
              (map (lambda (candidate)
                     (if (marlin-policy-object-matches?
                          candidate
                          family-value
                          target-id-value)
                       replacement-value
                       candidate))
                   object-values)
              object-values)
            matched-count: (length matched)
            conflict-reasons: conflict-reasons-value)))
     (else
      (error "unknown marlin policy object operation" operation-name)))))

;;; Boundary: Operation counters are scalar receipt fields, not branch side effects.
;; MarlinResult <- MarlinInput
(def (marlin-policy-operation-delta operation-name expected-name)
  (if (string=? operation-name expected-name) 1 0))

;;; Boundary: One fold step applies object surgery and accumulates receipts.
;; MarlinResult <- MarlinInput
(def (marlin-policy-pack-apply-operation-step operation-value state)
  (let* ((operation-name (.get operation-value operation))
         (operation-result
          (marlin-policy-pack-apply-operation
           (.get state policy-objects)
           operation-value))
         (receipt
          (marlin-policy-object-surgery-receipt
           operation-value
           (.get operation-result matched-count)
           (.get operation-result conflict-reasons)))
         (operation-conflict?
          (not (null? (.get operation-result conflict-reasons)))))
    (.o policy-objects: (.get operation-result policy-objects)
        surgery-receipts: (cons receipt (.get state surgery-receipts))
        add-operation-count:
        (+ (.get state add-operation-count)
           (marlin-policy-operation-delta operation-name "add"))
        remove-operation-count:
        (+ (.get state remove-operation-count)
           (marlin-policy-operation-delta operation-name "remove"))
        disable-operation-count:
        (+ (.get state disable-operation-count)
           (marlin-policy-operation-delta operation-name "disable"))
        replace-operation-count:
        (+ (.get state replace-operation-count)
           (marlin-policy-operation-delta operation-name "replace"))
        matched-surgery-receipt-count:
        (+ (.get state matched-surgery-receipt-count)
           (if (> (.get operation-result matched-count) 0) 1 0))
        conflict-surgery-receipt-count:
        (+ (.get state conflict-surgery-receipt-count)
           (if operation-conflict? 1 0)))))

;;; Boundary: Pack object surgery is deterministic and receipt-producing.
;; MarlinResult <- MarlinInput
(def (marlin-policy-pack-apply-operations object-values operation-values)
  (let (state
        (foldl marlin-policy-pack-apply-operation-step
               (.o policy-objects: object-values
                   surgery-receipts: '()
                   add-operation-count: 0
                   remove-operation-count: 0
                   disable-operation-count: 0
                   replace-operation-count: 0
                   matched-surgery-receipt-count: 0
                   conflict-surgery-receipt-count: 0)
               operation-values))
    (.o policy-objects: (.get state policy-objects)
        surgery-receipts: (reverse (.get state surgery-receipts))
        add-operation-count: (.get state add-operation-count)
        remove-operation-count: (.get state remove-operation-count)
        disable-operation-count: (.get state disable-operation-count)
        replace-operation-count: (.get state replace-operation-count)
        matched-surgery-receipt-count:
        (.get state matched-surgery-receipt-count)
        conflict-surgery-receipt-count:
        (.get state conflict-surgery-receipt-count))))

;;; Boundary: Conflict reason family counts are typed projection scalars.
;; MarlinResult <- MarlinInput
(def (marlin-policy-surgery-conflict-reason-count receipt-values reason-value)
  (length
   (filter (lambda (receipt-value)
             (member reason-value (.get receipt-value conflict-reasons)))
           receipt-values)))

;;; Boundary: Disabled object counts keep object surgery auditable.
;; MarlinResult <- MarlinInput
(def (marlin-policy-disabled-object-count object-values)
  (length
   (filter (lambda (object-value)
             (and (marlin-policy-object? object-value)
                  (.get object-value policy-object-disabled)))
           object-values)))
