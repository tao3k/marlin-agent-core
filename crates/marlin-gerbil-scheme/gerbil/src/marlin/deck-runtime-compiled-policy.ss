;;; -*- Gerbil -*-
;;; Macro-specialized Deck runtime policy selectors.

package: marlin

(import :marlin/deck-runtime)

(export marlin-deck-runtime-compiled-policy-kind
        marlin-deck-runtime-compiled-policy-capability-names
        defmarlin-deck-runtime-compiled-route-selector
        defmarlin-deck-runtime-cached-compiled-route-selector
        defmarlin-deck-runtime-cached-compiled-route-index-selector
        defmarlin-deck-runtime-direct-compiled-route-index-selector)

(def marlin-deck-runtime-compiled-policy-kind
  "marlin-deck-runtime.compiled-policy.v1")

(def (marlin-deck-runtime-compiled-policy-capability-names)
  '("compiled-macro-selector"
    "ahead-of-time-policy-shape"
    "direct-branch-dispatch"
    "cached-policy-template"
    "policy-index-selector"
    "direct-policy-index-selector"))

(def (compiled-policy-string-prefix? prefix value)
  (string-prefix? prefix value))

(def (compiled-policy-any-prefix? prefixes value)
  (let loop ((remaining prefixes))
    (cond
      ((null? remaining) #f)
      ((compiled-policy-string-prefix? (car remaining) value) #t)
      (else (loop (cdr remaining))))))

(def (compiled-policy-string-member? value values)
  (if (member value values) #t #f))

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
         (let loop ((remaining compiled-routes))
           (cond
             ((null? remaining) #f)
             ((and
               (compiled-policy-any-prefix? (car (car remaining)) request-command)
               (compiled-policy-string-member?
                request-agent-scope
                (cadr (car remaining))))
              (caddr (car remaining)))
             (else (loop (cdr remaining))))))))))

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
         (let loop ((remaining compiled-routes))
           (cond
             ((null? remaining) #f)
             ((and
               (compiled-policy-any-prefix? (car (car remaining)) request-command)
               (compiled-policy-string-member?
                request-agent-scope
                (cadr (car remaining))))
              (caddr (car remaining)))
             (else (loop (cdr remaining))))))))))

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
