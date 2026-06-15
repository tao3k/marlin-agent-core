;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; Macro-specialized Deck runtime policy selectors.

package: marlin

(import :marlin/deck-runtime)

(export marlin-deck-runtime-compiled-policy-kind
        marlin-deck-runtime-compiled-policy-capability-names
        defmarlin-deck-runtime-compiled-route-selector
        defmarlin-deck-runtime-cached-compiled-route-selector
        defmarlin-deck-runtime-cached-compiled-route-index-selector
        defmarlin-deck-runtime-direct-compiled-route-index-selector)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-compiled-policy-kind
  "marlin-deck-runtime.compiled-policy.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-compiled-policy-capability-names)
  '("compiled-macro-selector"
    "ahead-of-time-policy-shape"
    "direct-branch-dispatch"
    "cached-policy-template"
    "policy-index-selector"
    "direct-policy-index-selector"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (compiled-policy-string-prefix? prefix value)
  (string-prefix? prefix value))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (compiled-policy-any-prefix? prefixes value)
  (ormap (cut compiled-policy-string-prefix? <> value) prefixes))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (compiled-policy-string-member? value values)
  (if (member value values) #t #f))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(defrules defmarlin-deck-runtime-compiled-route-selector ()
  ((_ binding
      (policy-name provider model
       (command-prefix ...)
       (agent-scope-value ...)
       context-mode isolation-mode)
      ...)
   (def (binding request-command request-agent-scope)
     (cond
       ((and
         (compiled-policy-any-prefix? (list command-prefix ...) request-command)
         (compiled-policy-string-member?
          request-agent-scope
          (list agent-scope-value ...)))
        (make-marlin-deck-runtime-model-route-policy
         policy-name
         provider
         model
         (list command-prefix ...)
         (list agent-scope-value ...)
         context-mode
         isolation-mode))
       ...
       (else #f)))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(defrules defmarlin-deck-runtime-cached-compiled-route-selector ()
  ((_ binding
      (policy-name provider model
       (command-prefix ...)
       (agent-scope-value ...)
       context-mode isolation-mode)
      ...)
   (def binding
     (let ((compiled-routes
            (list
             (list
              (list command-prefix ...)
              (list agent-scope-value ...)
              (make-marlin-deck-runtime-model-route-policy
               policy-name
               provider
               model
               (list command-prefix ...)
               (list agent-scope-value ...)
               context-mode
               isolation-mode))
             ...)))
       (lambda (request-command request-agent-scope)
         (let ((matched-route
                (find
                 (lambda (route)
                   (and
                    (compiled-policy-any-prefix? (car route) request-command)
                    (compiled-policy-string-member?
                     request-agent-scope
                     (cadr route))))
                 compiled-routes)))
           (if matched-route (caddr matched-route) #f)))))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(defrules defmarlin-deck-runtime-cached-compiled-route-index-selector ()
  ((_ binding
      (policy-index
       (command-prefix ...)
       (agent-scope-value ...))
      ...)
   (def binding
     (let ((compiled-routes
            (list
             (list
              (list command-prefix ...)
              (list agent-scope-value ...)
              policy-index)
             ...)))
       (lambda (request-command request-agent-scope)
         (let ((matched-route
                (find
                 (lambda (route)
                   (and
                    (compiled-policy-any-prefix? (car route) request-command)
                    (compiled-policy-string-member?
                     request-agent-scope
                     (cadr route))))
                 compiled-routes)))
           (if matched-route (caddr matched-route) #f)))))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(defrules defmarlin-deck-runtime-direct-compiled-route-index-selector ()
  ((_ binding
      (policy-index
       (command-prefix ...)
       (agent-scope-value ...))
      ...)
   (def (binding request-command request-agent-scope)
     (cond
       ((and
         (or (compiled-policy-string-prefix? command-prefix request-command) ...)
         (or (string=? request-agent-scope agent-scope-value) ...))
        policy-index)
       ...
       (else #f)))))
