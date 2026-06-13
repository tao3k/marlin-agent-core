;;; -*- Gerbil -*-
;;; JSON adapter for the Marlin Deck runtime model-route policy selector.

package: marlin

(import :std/text/json
        :marlin/deck-runtime)

(export read-marlin-deck-runtime-policy-request
        run-marlin-deck-runtime-policy-adapter
        main)

(def (read-all input)
  (let loop ((acc ""))
    (let ((ch (read-char input)))
      (if (eof-object? ch)
        acc
        (loop (string-append acc (string ch)))))))

(def (read-marlin-deck-runtime-policy-request)
  (string->json-object (read-all (current-input-port))))

(def (required-field object field)
  (let ((value (hash-ref object field #f)))
    (unless value
      (error "marlin deck runtime policy request missing required field" field))
    value))

(def (policy-object policy)
  (make-marlin-deck-runtime-model-route-policy
   (required-field policy "name")
   (required-field policy "provider")
   (required-field policy "model")
   (required-field policy "command_prefixes")
   (required-field policy "agent_scopes")
   (required-field policy "context_mode")
   (required-field policy "isolation_mode")))

(def (policy-objects policies)
  (let loop ((remaining policies) (acc '()))
    (if (null? remaining)
      (reverse acc)
      (loop (cdr remaining) (cons (policy-object (car remaining)) acc)))))

(def (display-json-string value)
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

(def (display-json-string-list values)
  (display "[")
  (let loop ((remaining values) (first #t))
    (if (null? remaining)
      #t
      (begin
        (if first #f (display ","))
        (display-json-string (car remaining))
        (loop (cdr remaining) #f))))
  (display "]"))

(def (display-json-bool value)
  (if value (display "true") (display "false")))

(def (display-marlin-deck-runtime-policy-adapter-policy policy)
  (display "{\"kind\":")
  (display-json-string (.get policy kind))
  (display ",\"name\":")
  (display-json-string (.get policy name))
  (display ",\"provider\":")
  (display-json-string (.get policy provider))
  (display ",\"model\":")
  (display-json-string (.get policy model))
  (display ",\"command_prefixes\":")
  (display-json-string-list (.get policy command-prefixes))
  (display ",\"agent_scopes\":")
  (display-json-string-list (.get policy agent-scopes))
  (display ",\"context_mode\":")
  (display-json-string (.get policy context-mode))
  (display ",\"isolation_mode\":")
  (display-json-string (.get policy isolation-mode))
  (display "}"))

(def (display-marlin-deck-runtime-policy-adapter-selection selection)
  (display "{\"schema_id\":")
  (display-json-string (.get selection kind))
  (display ",\"command\":")
  (display-json-string (.get selection command))
  (display ",\"agent_scope\":")
  (display-json-string (.get selection agent-scope))
  (display ",\"matched\":")
  (display-json-bool (.get selection matched))
  (display ",\"policy\":")
  (let ((policy (.get selection policy)))
    (if policy
      (display-marlin-deck-runtime-policy-adapter-policy policy)
      (display "null")))
  (display "}"))

(def (run-marlin-deck-runtime-policy-adapter)
  (let* ((request (read-marlin-deck-runtime-policy-request))
         (policies (policy-objects (required-field request "policies")))
         (command (required-field request "command"))
         (agent-scope (required-field request "agent_scope")))
    (display-marlin-deck-runtime-policy-adapter-selection
     (marlin-deck-runtime-model-route-selection
      policies command agent-scope))
    (newline)))

(def (main . _args)
  (run-marlin-deck-runtime-policy-adapter))
