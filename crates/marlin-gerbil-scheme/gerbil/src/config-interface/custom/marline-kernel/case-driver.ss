;;; -*- Gerbil -*-
;;; Boundary: Marlin kernel case driver projection.
;;; Invariant: Scheme declares typed case intent; Rust owns runtime execution.

package: config-interface/custom/marline-kernel

(import :clan/poo/object
        :config-interface/modules/lib
        (only-in :config-interface/modules/policy-pack
                 marlinLoopVerticalMainlineProjectionDescriptors
                 marlinPolicyCombinationMatrixSlotMergeAlgebraReceipts)
        :config-interface/modules/prefabs/user-interface
        (only-in :config-interface/custom/marline-kernel/config
                 poo-flow-custom-module-runtime-handoff-llm-case
                 poo-flow-custom-module-policy-receipt-gate-llm-case
                 poo-flow-custom-module-loop-contract-llm-case
                 poo-flow-custom-module-failure-retry-llm-case))

(export +marline-kernel-loop-case-driver-receipt-kind+
        marline-kernel-loop-case-driver-receipts
        marline-kernel-loop-case-driver-module-receipts
        marline-kernel-loop-case-driver-vertical-receipts
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

(def (marline-kernel-module-head module-bundle)
  (car module-bundle))

(def (marline-kernel-module-kind module-bundle)
  (.get (marline-kernel-module-head module-bundle) kind))

(def (marline-kernel-module-user-module module-bundle)
  (.get (marline-kernel-module-head module-bundle) user-module))

(def (marline-kernel-module-selection-flags module-bundle)
  (.get (marline-kernel-module-head module-bundle) selection-flags))

(def (marline-kernel-module-source-ref module-bundle)
  (.get (marline-kernel-module-head module-bundle) source-ref))

(def (marline-kernel-module-entrypoint module-bundle)
  (.get (marline-kernel-module-head module-bundle) entrypoint))

(def (marline-kernel-module-enabled? module-bundle)
  (.get (marline-kernel-module-head module-bundle) enabled?))

(def (marline-kernel-symbol-prefix values)
  (cond
   ((null? values) '())
   ((symbol? (car values))
    (cons (car values) (marline-kernel-symbol-prefix (cdr values))))
   (else '())))

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

(def (marline-kernel-loop-case-vertical-descriptors/list)
  (vector->list (marlinLoopVerticalMainlineProjectionDescriptors)))

(def (marline-kernel-loop-case-vertical-case-id vertical-spec)
  (.get vertical-spec vertical-case-id))

(def (marline-kernel-loop-case-vertical-profile-id vertical-spec)
  (.get vertical-spec profile-id))

(def (marline-kernel-loop-case-vertical-tags vertical-spec)
  (vector->list (.get vertical-spec vertical-capability-tags)))

(def (marline-kernel-loop-case-vertical-compiler-receipt vertical-spec)
  (.get vertical-spec compiler-receipt))

(def (marline-kernel-loop-case-vertical-tag? vertical-spec tag)
  (if (member tag (marline-kernel-loop-case-vertical-tags vertical-spec))
    #t
    #f))

(def (marline-kernel-loop-case-live-llm-required? vertical-spec)
  (marline-kernel-loop-case-vertical-tag? vertical-spec '+tool-repair))

(def (marline-kernel-loop-case-live-gate-env vertical-spec)
  (if (marline-kernel-loop-case-live-llm-required? vertical-spec)
    "MARLIN_LIVE_LLM"
    "none"))

(def (marline-kernel-loop-case-live-llm-denial-receipt vertical-spec)
  (if (marline-kernel-loop-case-live-llm-required? vertical-spec)
    'deferred-no-live-llm
    'not-required))

(def (marline-kernel-loop-case-llm-repair-intent vertical-spec)
  (if (marline-kernel-loop-case-live-llm-required? vertical-spec)
    'single-file-repair
    'none))

(def (marline-kernel-loop-case-tool-intent-count vertical-spec)
  (if (or (marline-kernel-loop-case-vertical-tag? vertical-spec '+tool-repair)
          (marline-kernel-loop-case-vertical-tag? vertical-spec '+tool-selection)
          (marline-kernel-loop-case-vertical-tag? vertical-spec '+sandbox)
          (marline-kernel-loop-case-vertical-tag? vertical-spec '+denylist)
          (marline-kernel-loop-case-vertical-tag? vertical-spec '+checker)
          (marline-kernel-loop-case-vertical-tag? vertical-spec '+dynamic-rewrite)
          (marline-kernel-loop-case-vertical-tag? vertical-spec '+repair)
          (marline-kernel-loop-case-vertical-tag? vertical-spec '+rewrite))
    1
    0))

(def (marline-kernel-loop-case-memory-intent-count vertical-spec)
  (if (or (marline-kernel-loop-case-vertical-tag? vertical-spec '+memory)
          (marline-kernel-loop-case-vertical-tag? vertical-spec '+memory-recall))
    1
    0))

(def (marline-kernel-loop-case-placement-intent-count vertical-spec)
  ;; Every vertical loop case reaches the runtime handoff lane.
  1)

(def (marline-kernel-loop-case-compiler-loop-program compiler-receipt)
  (.get compiler-receipt loop-program))

(def (marline-kernel-loop-case-compiler-resolved-policy-pack compiler-receipt)
  (.get compiler-receipt resolved-policy-pack))

(def (marline-kernel-loop-case-boundary-token value)
  (cond
   ((equal? value "scheme-types-to-rust-types")
    'scheme-types->rust-types)
   ((equal? value "rust-owned-cli-trace-cross-process")
    'rust-owned-cli-trace-cross-process)
   (else value)))

(def (marline-kernel-loop-case-scalar->string value)
  (cond
   ((string? value) value)
   ((symbol? value) (symbol->string value))
   ((number? value) (number->string value))
   (else
    (error "unsupported loop case trace scalar" value))))

(def (marline-kernel-loop-case-join values separator)
  (cond
   ((null? values) "")
   ((null? (cdr values))
    (marline-kernel-loop-case-scalar->string (car values)))
   (else
    (string-append
     (marline-kernel-loop-case-scalar->string (car values))
     separator
     (marline-kernel-loop-case-join (cdr values) separator)))))

(def (marline-kernel-loop-case-transition-slot transition field)
  (cond
   ((equal? field 'action)
    (.get transition action))
   ((equal? field 'event)
    (.get transition event))
   (else
    (error "unsupported loop case transition field" field))))

(def (marline-kernel-loop-case-transition-field-string transitions field)
  (marline-kernel-loop-case-join
   (map (lambda (transition)
          (marline-kernel-loop-case-transition-slot transition field))
        (vector->list transitions))
   "|"))

(def (marline-kernel-loop-case-policy-digest-string loop-program)
  (marline-kernel-loop-case-join
   (vector->list (.get loop-program policy_digest))
   ","))

(def (marline-kernel-loop-case-mechanism-policy-string mechanism-policies)
  (marline-kernel-loop-case-join
   (vector->list mechanism-policies)
   "|"))

(def (marline-kernel-loop-case-merge-receipt-field-string merge-receipts field)
  (marline-kernel-loop-case-join
   (map (lambda (merge-receipt)
          (.ref merge-receipt field))
        (vector->list merge-receipts))
   "|"))

(def (marline-kernel-loop-case-merge-receipt-status-count merge-receipts status-value)
  (length
   (filter (lambda (merge-receipt)
             (equal? (.ref merge-receipt 'status) status-value))
           (vector->list merge-receipts))))

(def (marline-kernel-loop-case-policy-mixin-stack-present? vertical-spec)
  (marline-kernel-loop-case-vertical-tag? vertical-spec '+policy-combination))

(def (marline-kernel-loop-case-policy-mixin-stack-law-string)
  (marline-kernel-loop-case-join
   (map (lambda (merge-receipt)
          (string-append (.get merge-receipt slot)
                         "="
                         (.get merge-receipt merge)))
        (vector->list
         (marlinPolicyCombinationMatrixSlotMergeAlgebraReceipts)))
   "|"))

(def (marline-kernel-loop-case-driver-vertical-receipt vertical-spec)
  (let* ((case-id (marline-kernel-loop-case-vertical-case-id vertical-spec))
         (profile-id (marline-kernel-loop-case-vertical-profile-id vertical-spec))
         (compiler-receipt
          (marline-kernel-loop-case-vertical-compiler-receipt vertical-spec))
         (resolved-policy-pack
          (marline-kernel-loop-case-compiler-resolved-policy-pack
           compiler-receipt))
         (loop-program
          (marline-kernel-loop-case-compiler-loop-program compiler-receipt))
         (hot-policy (.get resolved-policy-pack hot))
         (audit-policy (.get resolved-policy-pack audit))
         (budget-caps (.get hot-policy budget_caps))
         (forced-slots (.get audit-policy forced_slots))
         (merge-receipts (.get audit-policy merge_receipts))
         (policy-mixin-stack-present?
          (marline-kernel-loop-case-policy-mixin-stack-present? vertical-spec))
         (mechanism-policies (.get loop-program mechanism_policies))
         (transitions (.get loop-program transitions))
         (command-vector
          (list "marlin" "loop" "program" "run"
                "--profile" profile-id)))
    `((kind . ,+marline-kernel-loop-case-driver-receipt-kind+)
      (case-id . ,case-id)
      (profile-ref . ,profile-id)
      (runtime-mode . typed-loop-projection)
      (live-gate-env . ,(marline-kernel-loop-case-live-gate-env vertical-spec))
      (live-enabled? . #f)
      (live-llm-required? .
                          ,(marline-kernel-loop-case-live-llm-required?
                            vertical-spec))
      (live-llm-allowed? . #f)
      (live-llm-denial-receipt .
                                ,(marline-kernel-loop-case-live-llm-denial-receipt
                                  vertical-spec))
      (llm-repair-intent .
                         ,(marline-kernel-loop-case-llm-repair-intent
                           vertical-spec))
      (session-transform . loop-policy-profile->loop-program)
      (tool-intent-count .
                         ,(marline-kernel-loop-case-tool-intent-count
                           vertical-spec))
      (memory-intent-count .
                           ,(marline-kernel-loop-case-memory-intent-count
                             vertical-spec))
      (placement-intent-count .
                              ,(marline-kernel-loop-case-placement-intent-count
                                vertical-spec))
      (runtime-handoff-kind . loop-program-runtime-handoff)
      (runtime-receipt-kind . loop-program-runtime-receipt)
      (derived-session-kind . derived-session/from-loop-receipt)
      (smoke-status . typed-loop-projection-ready)
      (command-kind . loop-program-run)
      (command-vector . ,command-vector)
      (input-path . none)
      (stable-fixture? . #f)
      (artifact-outputs . ("resolved-policy-pack" "loop-program"))
      (result-protocol . ,(.get compiler-receipt kind))
      (observability . ,case-id)
      (observes . ,(marline-kernel-loop-case-vertical-tags vertical-spec))
      (capability-tags . ,(marline-kernel-loop-case-vertical-tags vertical-spec))
      (metadata .
                ((owner . "marlin")
                 (surface . "config-interface-loop-policy-profile")))
      (policy-owner . ,+marline-kernel-loop-case-driver-policy-owner+)
      (control-plane-owner . ,+marline-kernel-loop-case-driver-control-plane-owner+)
      (runtime-execution-owner . ,+marline-kernel-loop-case-driver-runtime-owner+)
      (module-kind . "marlin.config-interface.loop-policy-profile-projection.v1")
      (module-user-module . funflow)
      (module-selection-tags .
                             ,(marline-kernel-loop-case-vertical-tags
                               vertical-spec))
      (module-source-ref . ,profile-id)
      (module-entrypoint . marlinLoopPolicyProfileCompilerReceipts)
      (module-enabled? . #t)
      (vertical-mainline? . #t)
      (compiler-owner . ,(.get compiler-receipt compiler-owner))
      (compiler-profile-id . ,(.get compiler-receipt profile-id))
      (resolved-policy-pack-policy-epoch .
                                          ,(.get resolved-policy-pack
                                                 policy_epoch))
      (loop-program-id . ,(.get loop-program program_id))
      (loop-program-policy-epoch . ,(.get loop-program policy_epoch))
      (transition-count . ,(vector-length transitions))
      (transition-actions .
                          ,(marline-kernel-loop-case-transition-field-string
                            transitions
                            'action))
      (transition-events .
                         ,(marline-kernel-loop-case-transition-field-string
                           transitions
                           'event))
      (mechanism-policy-count . ,(vector-length mechanism-policies))
      (mechanism-policy-ids .
                            ,(marline-kernel-loop-case-mechanism-policy-string
                              mechanism-policies))
      (mechanism-policies . ,(vector->list mechanism-policies))
      (policy-digest-length . ,(vector-length
                                (.get loop-program policy_digest)))
      (policy-digest-octets .
                            ,(marline-kernel-loop-case-policy-digest-string
                              loop-program))
      (capability-mask . ,(.get hot-policy capability_mask))
      (budget-max-attempts . ,(.get budget-caps max_attempts))
      (budget-max-cost-units . ,(.get budget-caps max_cost_units))
      (budget-max-wall-time-ms . ,(.get budget-caps max_wall_time_ms))
      (policy-forced-slot-count . ,(vector-length forced-slots))
      (policy-merge-receipt-count . ,(vector-length merge-receipts))
      (policy-conflict-merge-receipt-count .
                                         ,(marline-kernel-loop-case-merge-receipt-status-count
                                           merge-receipts
                                           "conflict"))
      (policy-merge-kinds .
                          ,(marline-kernel-loop-case-merge-receipt-field-string
                            merge-receipts
                            'merge))
      (policy-merge-statuses .
                             ,(marline-kernel-loop-case-merge-receipt-field-string
                               merge-receipts
                               'status))
      (policy-mixin-stack-present? . ,policy-mixin-stack-present?)
      (policy-mixin-stack-receipt-kind .
                                       ,(if policy-mixin-stack-present?
                                          "marlin.config-interface.policy-pack.mixin-stack-compiler-receipt.v1"
                                          "none"))
      (policy-mixin-stack-profile-id .
                                     ,(if policy-mixin-stack-present?
                                        profile-id
                                        "none"))
      (policy-mixin-stack-mixin-count .
                                     ,(if policy-mixin-stack-present?
                                        (vector-length
                                         (.get audit-policy policy_mixins))
                                        0))
      (policy-mixin-stack-slot-merge-law-count .
                                              ,(if policy-mixin-stack-present?
                                                 (vector-length merge-receipts)
                                                 0))
      (policy-mixin-stack-slot-merge-laws .
                                         ,(if policy-mixin-stack-present?
                                            (marline-kernel-loop-case-policy-mixin-stack-law-string)
                                            ""))
      (policy-mixin-stack-linearization-owner .
                                             ,(if policy-mixin-stack-present?
                                                "poo-flow.c3-c4"
                                                "none"))
      (policy-mixin-stack-slot-merge-owner .
                                         ,(if policy-mixin-stack-present?
                                            "poo-flow.slot-merge-algebra"
                                            "none"))
      (scheme-boundary .
                       ,(marline-kernel-loop-case-boundary-token
                         (.get compiler-receipt scheme-boundary)))
      (serialization-boundary .
                              ,(marline-kernel-loop-case-boundary-token
                                (.get compiler-receipt
                                      serialization-boundary))))))

(def (marline-kernel-loop-case-driver-live-enabled? live-gate-env)
  (let (value (getenv live-gate-env #f))
    (and value
         (not (member value '("" "0" "false" "FALSE" "no" "NO"))))))

(def (marline-kernel-loop-case-driver-smoke-status live-enabled?)
  (if live-enabled?
    'live-enabled
    'no-live-llm-denied))

(def (marline-kernel-loop-case-driver-receipt module-bundle)
  (let* ((case (marline-kernel-module-case module-bundle))
         (module-selection-flags
          (marline-kernel-module-selection-flags module-bundle))
         (check (marline-kernel-case-check case))
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
      (module-kind . ,(marline-kernel-module-kind module-bundle))
      (module-user-module . ,(marline-kernel-module-user-module module-bundle))
      (module-selection-tags . ,(marline-kernel-symbol-prefix module-selection-flags))
      (module-source-ref . ,(marline-kernel-module-source-ref module-bundle))
      (module-entrypoint . ,(marline-kernel-module-entrypoint module-bundle))
      (module-enabled? . ,(marline-kernel-module-enabled? module-bundle))
      (scheme-boundary . scheme-types->rust-types)
      (serialization-boundary . rust-owned-cli-trace-cross-process))))

(def (marline-kernel-loop-case-driver-module-receipts)
  (map (lambda (module-bundle)
         (marline-kernel-loop-case-driver-receipt module-bundle))
       +marline-kernel-loop-case-bundles+))

(def (marline-kernel-loop-case-driver-vertical-receipts)
  (map marline-kernel-loop-case-driver-vertical-receipt
       (marline-kernel-loop-case-vertical-descriptors/list)))

(def (marline-kernel-loop-case-driver-receipts)
  (append (marline-kernel-loop-case-driver-module-receipts)
          (marline-kernel-loop-case-driver-vertical-receipts)))
