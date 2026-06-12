;;; -*- Gerbil -*-
;;; Library-module entry point for configured Marlin hook policy procedures.

package: marlin

(import :std/text/json)

(export read-marlin-hook-policy-invocation
        run-marlin-hook-policy-adapter
        main)

(def (read-all input)
  (let loop ((acc ""))
    (let ((ch (read-char input)))
      (if (eof-object? ch)
        acc
        (loop (string-append acc (string ch)))))))

(def (read-marlin-hook-policy-invocation)
  (string->json-object (read-all (current-input-port))))

(def (required-hook-policy-field invocation field)
  (let ((value (hash-ref invocation field #f)))
    (unless value
      (error "marlin hook policy invocation missing required field" field))
    value))

(def (hook-policy-module-symbol module-name)
  (if (and (> (string-length module-name) 0)
           (char=? (string-ref module-name 0) #\:))
    (string->symbol module-name)
    (string->symbol (string-append ":" module-name))))

(def (call-marlin-hook-policy-procedure invocation)
  (let* ((module-name (required-hook-policy-field invocation "module"))
         (procedure-name (required-hook-policy-field invocation "procedure"))
         (request-json (required-hook-policy-field invocation "request_json"))
         (_ (eval (list 'import (hook-policy-module-symbol module-name))))
         (policy-procedure (eval (string->symbol procedure-name))))
    (policy-procedure request-json)))

(def (display-marlin-hook-policy-result result)
  (cond
    ((string? result) (display result))
    ((hash-table? result) (display (json-object->string result)))
    (else (error "marlin hook policy procedure returned unsupported result" result))))

(def (run-marlin-hook-policy-adapter)
  (display-marlin-hook-policy-result
   (call-marlin-hook-policy-procedure
    (read-marlin-hook-policy-invocation)))
  (newline))

(def (main . _args)
  (run-marlin-hook-policy-adapter))
