;;; -*- Gerbil -*-
;;; Boundary: Driver smoke for the canonical Marlin Scheme config interface.

(import :std/misc/process)

;;; Boundary: Keep build.ss verification inside Gerbil instead of shell glue.
;; MarlinResult <- MarlinInput
(def (driver-output->datum output)
  (read (open-input-string output)))

;;; Boundary: Local checks stay scalar around process output.
;; MarlinResult <- MarlinInput
(defrules check ()
  ((_ expression => expected)
   (let ((actual-value expression)
         (expected-value expected))
     (unless (equal? actual-value expected-value)
       (error "check failed" actual-value expected-value)))))

(def (contains? values value)
  (if (member value values) #t #f))

(def (build-driver . args)
  (driver-output->datum
   (run-process (cons "gxi" (cons "build.ss" args)))))

(def build-driver-meta
  (build-driver "meta"))

(def build-driver-spec
  (build-driver "spec"))

(check build-driver-meta => '("spec" "compile" "clean"))
(check (contains? build-driver-spec "config-interface/init.ss") => #t)
(check (contains? build-driver-spec "config-interface/lib.ss") => #t)
(check (contains? build-driver-spec "config-interface/modules/lib.ss") => #t)
(check (contains? build-driver-spec "config-interface/modules/policy-pack.ss") => #t)
(check (contains? build-driver-spec "config-interface/modules/policy-object.ss") => #t)
(check (contains? build-driver-spec "config-interface/modules/loop-engine-policy.ss") => #t)
(check (contains? build-driver-spec "config-interface/modules/memory-policy.ss") => #t)
(check (contains? build-driver-spec "config-interface/modules/model-route-policy.ss") => #t)
(check (contains? build-driver-spec
                  "config-interface/modules/prefabs/user-interface-loop-policy.ss")
       => #t)
(check (contains? build-driver-spec "config-interface/custom/marline-kernel/config.ss")
       => #t)
(check (contains? build-driver-spec "../examples/user-interface-module-config/init.ss")
       => #f)

(display "config-interface-driver-ok")
(newline)
(display "config-interface-driver-targets=")
(display (length build-driver-spec))
(newline)
