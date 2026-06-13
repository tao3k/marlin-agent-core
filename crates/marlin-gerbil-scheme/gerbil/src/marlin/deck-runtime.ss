;;; -*- Gerbil -*-
;;; Marlin Deck runtime capability bridge exposed to Rust tests.

package: marlin

(import :clan/poo/object)

(export marlin-deck-runtime-package-name
        marlin-deck-runtime-module
        marlin-deck-runtime-poo-dependency
        marlin-deck-runtime-poo-package-name
        marlin-deck-runtime-model-route-policy-kind
        marlin-deck-runtime-model-route-selection-kind
        marlin-deck-runtime-poo-module-names
        marlin-deck-runtime-poo-form-names
        marlin-deck-runtime-capability-names
        marlin-deck-runtime-rust-contract-names
        marlin-deck-runtime-policy-primitive-names
        marlin-deck-runtime-object-model-slot-names
        marlin-deck-runtime-capability?
        marlin-deck-runtime-poo-form?
        make-marlin-deck-runtime-model-route-policy
        marlin-deck-runtime-route-policy-match?
        marlin-deck-runtime-select-model-route-policy
        marlin-deck-runtime-model-route-selection
        marlin-deck-runtime-capability-fact
        marlin-deck-runtime-object-model-fact)

(def marlin-deck-runtime-package-name "marlin-deck-runtime")
(def marlin-deck-runtime-module ":marlin/deck-runtime")
(def marlin-deck-runtime-poo-dependency
  "git.cons.io/mighty-gerbils/gerbil-poo")
(def marlin-deck-runtime-poo-package-name "clan/poo")
(def marlin-deck-runtime-model-route-policy-kind
  "marlin-deck-runtime.model-route-policy.v1")
(def marlin-deck-runtime-model-route-selection-kind
  "marlin-deck-runtime.model-route-selection.v1")

(def (marlin-deck-runtime-poo-module-names)
  '(":clan/poo/object"
    ":clan/poo/mop"
    ":clan/poo/proto"))

(def (marlin-deck-runtime-poo-form-names)
  '(".o"
    ".def"
    ".get"
    ".ref"
    ".mix"
    ".defgeneric"
    "defmethod"
    "compose-proto"))

(def (marlin-deck-runtime-capability-names)
  '("rust-bridge"
    "runtime-assets"
    "hook-policy"
    "scheme-policy-runtime"
    "scheme-compiled-policy-macro"
    "scheme-complex-strategy"
    "dynamic-hook-policy"
    "model-route-policy"
    "poo-object-system"))

(def (marlin-deck-runtime-rust-contract-names)
  '("runtime-assets"
    "real-gxi"
    "typed-native-abi"))

(def (marlin-deck-runtime-policy-primitive-names)
  '("provider-id"
    "model-name"
    "command-prefixes"
    "agent-scopes"
    "context-mode"
    "isolation-mode"
    "session"
    "agent-lineage"
    "workspace-state"
    "org-memory"
    "hook-action"
    "customer-agent-class"
    "scheme-selector"))

(def (marlin-deck-runtime-object-model-slot-names)
  '("package"
    "module"
    "poo_dependency"
    "poo_package"
    "poo_modules"
    "poo_forms"
    "policy_primitives"
    "rust_contracts"))

(def (marlin-deck-runtime-capability? name)
  (if (member name (marlin-deck-runtime-capability-names)) #t #f))

(def (marlin-deck-runtime-poo-form? name)
  (if (member name (marlin-deck-runtime-poo-form-names)) #t #f))

(def (make-marlin-deck-runtime-model-route-policy
      policy-name-value
      provider-value
      model-value
      command-prefix-values
      agent-scope-values
      context-mode-value
      isolation-mode-value)
  (.o kind: marlin-deck-runtime-model-route-policy-kind
      name: policy-name-value
      provider: provider-value
      model: model-value
      command-prefixes: command-prefix-values
      agent-scopes: agent-scope-values
      context-mode: context-mode-value
      isolation-mode: isolation-mode-value))

(def (string-prefix? prefix value)
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

(def (string-member? value values)
  (if (member value values) #t #f))

(def (any-string-prefix? prefixes value)
  (let loop ((remaining prefixes))
    (cond
      ((null? remaining) #f)
      ((string-prefix? (car remaining) value) #t)
      (else (loop (cdr remaining))))))

(def (marlin-deck-runtime-route-policy-match? policy command agent-scope)
  (and (string=? (.get policy kind) marlin-deck-runtime-model-route-policy-kind)
       (any-string-prefix? (.get policy command-prefixes) command)
       (string-member? agent-scope (.get policy agent-scopes))))

(def (marlin-deck-runtime-select-model-route-policy policies command agent-scope)
  (let loop ((remaining policies))
    (cond
      ((null? remaining) #f)
      ((marlin-deck-runtime-route-policy-match? (car remaining) command agent-scope)
       (car remaining))
      (else (loop (cdr remaining))))))

(def (marlin-deck-runtime-model-route-selection policies command agent-scope)
  (let ((selected-policy (marlin-deck-runtime-select-model-route-policy
                          policies command agent-scope)))
    (.o kind: marlin-deck-runtime-model-route-selection-kind
        command: command
        agent-scope: agent-scope
        matched: (if selected-policy #t #f)
        policy: selected-policy)))

(def (marlin-deck-runtime-capability-fact)
  (list marlin-deck-runtime-package-name
        marlin-deck-runtime-module
        marlin-deck-runtime-poo-dependency
        marlin-deck-runtime-poo-package-name
        (marlin-deck-runtime-poo-module-names)
        (marlin-deck-runtime-poo-form-names)
        (marlin-deck-runtime-capability-names)
        (marlin-deck-runtime-policy-primitive-names)
        (marlin-deck-runtime-rust-contract-names)))

(def (marlin-deck-runtime-object-model-fact)
  (list "marlin-deck-runtime.object-model.v1"
        (marlin-deck-runtime-object-model-slot-names)
        (marlin-deck-runtime-poo-module-names)
        (marlin-deck-runtime-poo-form-names)
        (marlin-deck-runtime-policy-primitive-names)))
