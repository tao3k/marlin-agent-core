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

(def (run-marlin-deck-runtime-policy-adapter)
  (let* ((request (read-marlin-deck-runtime-policy-request))
         (policies (policy-objects (required-field request "policies")))
         (command (required-field request "command"))
         (agent-scope (required-field request "agent_scope")))
    (display-marlin-deck-runtime-model-route-selection-json
     policies command agent-scope)
    (newline)))

(def (main . _args)
  (run-marlin-deck-runtime-policy-adapter))
