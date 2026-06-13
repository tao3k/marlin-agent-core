;;; -*- Gerbil -*-
;;; Macro-specialized Deck runtime policy selectors.

package: marlin

(import :marlin/deck-runtime)

(export marlin-deck-runtime-compiled-policy-kind
        marlin-deck-runtime-compiled-policy-capability-names
        defmarlin-deck-runtime-compiled-route-selector
        defmarlin-deck-runtime-cached-compiled-route-selector
        display-marlin-deck-runtime-compiled-selection-json)

(def marlin-deck-runtime-compiled-policy-kind
  "marlin-deck-runtime.compiled-policy.v1")

(def (marlin-deck-runtime-compiled-policy-capability-names)
  '("compiled-macro-selector"
    "ahead-of-time-policy-shape"
    "direct-branch-dispatch"
    "cached-policy-template"
    "rust-json-compatible-selection"))

(def (compiled-policy-string-prefix? prefix value)
  (let ((prefix-length (string-length prefix))
        (value-length (string-length value)))
    (if (> prefix-length value-length)
      #f
      (let loop ((index 0))
        (cond
          ((= index prefix-length) #t)
          ((char=? (string-ref prefix index) (string-ref value index))
           (loop (+ index 1)))
          (else #f))))))

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

(def (display-compiled-policy-json-string value)
  (display "\"")
  (let ((value-length (string-length value)))
    (let loop ((index 0))
      (if (< index value-length)
        (begin
          (let ((ch (string-ref value index)))
            (cond
              ((char=? ch #\") (display "\\\""))
              ((char=? ch #\\) (display "\\\\"))
              ((char=? ch #\newline) (display "\\n"))
              ((char=? ch #\tab) (display "\\t"))
              (else (display ch))))
          (loop (+ index 1)))
        #t)))
  (display "\""))

(def (display-compiled-policy-json-string-list values)
  (display "[")
  (let loop ((remaining values) (first #t))
    (if (null? remaining)
      #t
      (begin
        (if first #f (display ","))
        (display-compiled-policy-json-string (car remaining))
        (loop (cdr remaining) #f))))
  (display "]"))

(def (display-compiled-policy-json-bool value)
  (if value (display "true") (display "false")))

(def (display-marlin-deck-runtime-compiled-selection-json selector command agent-scope)
  (let ((policy (selector command agent-scope)))
    (display "{\"schema_id\":")
    (display-compiled-policy-json-string
     marlin-deck-runtime-model-route-selection-kind)
    (display ",\"compiled_policy_schema\":")
    (display-compiled-policy-json-string
     marlin-deck-runtime-compiled-policy-kind)
    (display ",\"command\":")
    (display-compiled-policy-json-string command)
    (display ",\"agent_scope\":")
    (display-compiled-policy-json-string agent-scope)
    (display ",\"matched\":")
    (display-compiled-policy-json-bool policy)
    (display ",\"capabilities\":")
    (display-compiled-policy-json-string-list
     (marlin-deck-runtime-compiled-policy-capability-names))
    (display ",\"policy\":")
    (if policy
      (display-marlin-deck-runtime-model-route-policy-json policy)
      (display "null"))
    (display "}")))
