;;; -*- Gerbil -*-
;;; Boundary: Marlin kernel case driver projection.
;;; Invariant: Scheme declares typed case intent; Rust owns runtime execution.

package: config-interface/custom/marline-kernel

(import :clan/poo/object
        :config-interface/modules/lib
        :config-interface/modules/prefabs/user-interface
        (only-in :config-interface/custom/marline-kernel/config
                 poo-flow-custom-module-runtime-handoff-llm-case
                 poo-flow-custom-module-policy-receipt-gate-llm-case
                 poo-flow-custom-module-loop-contract-llm-case
                 poo-flow-custom-module-failure-retry-llm-case))

(export +marline-kernel-loop-case-driver-receipt-kind+
        marline-kernel-loop-case-driver-receipts
        marline-kernel-loop-case-driver-receipt
        marline-kernel-loop-case-driver-live-enabled?)

(def +marline-kernel-loop-case-driver-receipt-kind+
  "marlin.config-interface.loop-case-driver-receipt.v1")

(def +marline-kernel-loop-case-driver-runtime-owner+
  "marlin-agent-core")

(def +marline-kernel-loop-case-driver-control-plane-owner+
  "poo-flow")

(def +marline-kernel-loop-case-driver-policy-owner+
  "marlin")

(def +marline-kernel-loop-case-bundles+
  (list poo-flow-custom-module-runtime-handoff-llm-case
        poo-flow-custom-module-policy-receipt-gate-llm-case
        poo-flow-custom-module-loop-contract-llm-case
        poo-flow-custom-module-failure-retry-llm-case))

(def +marline-kernel-loop-case-fixture-paths+
  '("t/fixtures/config-interface/loop-cases/runtime-handoff-llm.loop-program-run.json"
    "t/fixtures/config-interface/loop-cases/policy-receipt-gate-llm.loop.json"
    "t/fixtures/config-interface/loop-cases/loop-contract-llm.loop.json"
    "t/fixtures/config-interface/loop-cases/failure-retry-llm.loop.json"))

(def (marline-kernel-module-case module-bundle)
  (car (UserInterfaceModuleBundleConfig module-bundle)))

(def (marline-kernel-case-check case)
  (car (.get case check-objects)))

(def (marline-kernel-metadata-ref object key)
  (let (value (assq key (.get object metadata)))
    (and value (cdr value))))

(def (marline-kernel-slot-ref/default object slot-name default-value)
  (if (member slot-name (.all-slots object))
    (.get object slot-name)
    default-value))

(def (marline-kernel-command-option-value command-vector option)
  (let loop ((rest command-vector))
    (cond
     ((null? rest) #f)
     ((and (pair? (cdr rest)) (equal? (car rest) option))
      (cadr rest))
     (else
      (loop (cdr rest))))))

(def (marline-kernel-list-prefix? prefix values)
  (cond
   ((null? prefix) #t)
   ((null? values) #f)
   ((equal? (car prefix) (car values))
    (marline-kernel-list-prefix? (cdr prefix) (cdr values)))
   (else #f)))

(def (marline-kernel-loop-command-kind command-vector)
  (cond
   ((marline-kernel-list-prefix? '("marlin" "loop" "program" "run") command-vector)
    'loop-program-run)
   ((marline-kernel-list-prefix? '("marlin" "loop" "run") command-vector)
    'loop-run)
   (else 'unknown)))

(def (marline-kernel-stable-fixture-path? path)
  (if (member path +marline-kernel-loop-case-fixture-paths+) #t #f))

(def (marline-kernel-loop-case-driver-live-enabled? live-gate-env)
  (let (value (getenv live-gate-env #f))
    (and value
         (not (member value '("" "0" "false" "FALSE" "no" "NO"))))))

(def (marline-kernel-loop-case-driver-smoke-status live-enabled?)
  (if live-enabled?
    'live-enabled
    'no-live-llm-denied))

(def (marline-kernel-loop-case-driver-receipt case)
  (let* ((check (marline-kernel-case-check case))
         (metadata (.get case metadata))
         (case-id (marline-kernel-metadata-ref case 'case-id))
         (profile-ref (marline-kernel-metadata-ref case 'profile-ref))
         (runtime-mode (marline-kernel-metadata-ref case 'runtime-mode))
         (live-gate-env (marline-kernel-metadata-ref case 'live-gate-env))
         (command-vector (.get check command-vector))
         (input-path (marline-kernel-command-option-value command-vector "--input"))
         (command-kind (marline-kernel-loop-command-kind command-vector))
         (live-enabled? (marline-kernel-loop-case-driver-live-enabled? live-gate-env)))
    `((kind . ,+marline-kernel-loop-case-driver-receipt-kind+)
      (case-id . ,case-id)
      (profile-ref . ,profile-ref)
      (runtime-mode . ,runtime-mode)
      (live-gate-env . ,live-gate-env)
      (live-enabled? . ,live-enabled?)
      (smoke-status . ,(marline-kernel-loop-case-driver-smoke-status live-enabled?))
      (command-kind . ,command-kind)
      (command-vector . ,command-vector)
      (input-path . ,input-path)
      (stable-fixture? . ,(marline-kernel-stable-fixture-path? input-path))
      (artifact-outputs . ,(.get check artifact-outputs))
      (result-protocol . ,(.get check result-protocol))
      (observability . ,(marline-kernel-slot-ref/default
                         check 'observability case-id))
      (observes . ,(marline-kernel-slot-ref/default check 'observes '()))
      (metadata . ,metadata)
      (policy-owner . ,+marline-kernel-loop-case-driver-policy-owner+)
      (control-plane-owner . ,+marline-kernel-loop-case-driver-control-plane-owner+)
      (runtime-execution-owner . ,+marline-kernel-loop-case-driver-runtime-owner+)
      (scheme-boundary . scheme-types->rust-types)
      (serialization-boundary . rust-owned-cli-trace-cross-process))))

(def (marline-kernel-loop-case-driver-receipts)
  (map (lambda (module-bundle)
         (marline-kernel-loop-case-driver-receipt
          (marline-kernel-module-case module-bundle)))
       +marline-kernel-loop-case-bundles+))
