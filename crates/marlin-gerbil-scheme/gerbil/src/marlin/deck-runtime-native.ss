;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy/runtime wrapper logic over the native ABI.

(import (only-in :std/foreign
                 begin-foreign
                 c-define
                 extern
                 int))

(include "./_deck-runtime-native.ssi")

(declare
  (block)
  (standard-bindings)
  (extended-bindings)
  (not safe))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define marlin-deck-runtime-native-abi-version 1)
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define marlin-deck-runtime-native-status-abi-mismatch 3)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (native-string-list len ref)
  (map ref (list-tabulate len identity)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (native-policy-command-prefixes policy)
  (native-string-list
   (native-policy-command-prefixes-len policy)
   (lambda (index) (native-policy-command-prefix-at policy index))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (native-policy-agent-scopes policy)
  (native-string-list
   (native-policy-agent-scopes-len policy)
   (lambda (index) (native-policy-agent-scope-at policy index))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (native-policy->policy policy)
  (list (native-policy-name policy)
        (native-policy-provider policy)
        (native-policy-model policy)
        (native-policy-command-prefixes policy)
        (native-policy-agent-scopes policy)
        (native-policy-context-mode policy)
        (native-policy-isolation-mode policy)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (native-request-policies request)
  (map (lambda (index)
         (native-policy->policy
          (native-request-policy-at request index)))
       (list-tabulate (native-request-policies-len request) identity)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (policy-name policy) (list-ref policy 0))
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (policy-provider policy) (list-ref policy 1))
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (policy-model policy) (list-ref policy 2))
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (policy-command-prefixes policy) (list-ref policy 3))
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (policy-agent-scopes policy) (list-ref policy 4))
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (policy-context-mode policy) (list-ref policy 5))
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (policy-isolation-mode policy) (list-ref policy 6))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (string-prefix? prefix value)
  (let ((prefix-length (string-length prefix))
        (value-length (string-length value)))
    (and (<= prefix-length value-length)
         (let loop ((index 0))
           (if (= index prefix-length)
             #t
             (and (char=? (string-ref prefix index)
                          (string-ref value index))
                  (loop (+ index 1))))))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (string-member? value values)
  (if (member value values) #t #f))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (any-string-prefix? prefixes value)
  (let loop ((remaining prefixes))
    (and (pair? remaining)
         (or (string-prefix? (car remaining) value)
             (loop (cdr remaining))))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (native-policy-command-prefix-match? policy command)
  (let ((len (native-policy-command-prefixes-len policy)))
    (let loop ((index 0))
      (and (< index len)
           (or (string-prefix?
                (native-policy-command-prefix-at policy index)
                command)
               (loop (+ index 1)))))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (native-policy-agent-scope-match? policy agent-scope)
  (let ((len (native-policy-agent-scopes-len policy)))
    (let loop ((index 0))
      (and (< index len)
           (or (string=? (native-policy-agent-scope-at policy index)
                         agent-scope)
               (loop (+ index 1)))))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (native-policy-match? policy command agent-scope)
  (and (native-policy-command-prefix-match? policy command)
       (native-policy-agent-scope-match? policy agent-scope)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (policy-match? policy command agent-scope)
  (and (any-string-prefix? (policy-command-prefixes policy) command)
       (string-member? agent-scope (policy-agent-scopes policy))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (select-policy-index policies command agent-scope)
  (let ((match
         (find
          (lambda (entry)
            (policy-match? (cdr entry) command agent-scope))
          (map cons
               (list-tabulate (length policies) identity)
               policies))))
    (if match (car match) #f)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (native-select-policy-index request command agent-scope)
  (let ((len (native-request-policies-len request)))
    (let loop ((index 0))
      (if (= index len)
        #f
        (let ((policy (native-request-policy-at request index)))
          (if (native-policy-match? policy command agent-scope)
            index
            (loop (+ index 1))))))))

(extern marlin-deck-runtime-select-model-route)
(begin-foreign
  (namespace ("marlin-deck-runtime/src/marlin/deck-runtime-native#"
              marlin-deck-runtime-native-abi-version
              native-request-policies
              native-request-command
              native-request-agent-scope
              native-select-policy-index
              select-policy-index
              native-set-selection!))

  (c-define (marlin-deck-runtime-select-model-route request selection)
    (deck-runtime-request* deck-runtime-selection*) int
    "marlin_deck_runtime_select_model_route" ""
    (if (not (= (native-request-abi-version request)
                marlin-deck-runtime-native-abi-version))
      marlin-deck-runtime-native-status-abi-mismatch
      (let ((policy-index
             (native-select-policy-index
              request
              (native-request-command request)
              (native-request-agent-scope request))))
        (native-set-selection!
         selection
         (if policy-index 1 0)
         (if policy-index policy-index -1))))))
