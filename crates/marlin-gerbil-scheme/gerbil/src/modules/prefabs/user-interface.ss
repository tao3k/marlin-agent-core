;;; -*- Gerbil -*-
;;; Boundary: Furnished user-interface prefab maintained by upstream Marlin.

package: modules/prefabs

(import (only-in :clan/poo/object .all-slots .get .o .ref)
        :marlin/deck-runtime
        :marlin/deck-runtime-condition-policy
        :marlin/deck-runtime-dynamic-hook
        :marlin/deck-runtime-extension-template
        :marlin/deck-runtime-loop-graph
        :marlin/deck-runtime-matcher
        :marlin/deck-runtime-script
        :marlin/deck-runtime-strategy
        :marlin/deck-runtime-user-module
        :marlin/deck-runtime-user-option
        :marlin/graph-loop-continuation-native-projection
        :modules/lib)

(export UserInterfaceWorkspace
        UserInterfaceWorkspaceCatalog
        UserInterfaceWorkspaceEvaluation
        UserInterfaceWorkspaceSystemPresentation
        UserInterfaceWorkspaceWorkflow
        UserInterfacePolicyPack
        UserInterfacePolicyPackCatalog
        UserInterfacePolicyPackPresentation
        UserInterfaceModuleConfig
        UserInterfaceWorkspaceProfile
        UserInterfaceAgentProfile
        UserInterfaceHookProfile
        UserInterfaceLoopContinuationProfile
        user-interface-loop-continuation-profile
        user-interface-loop-continuation-projection
        user-interface-subagent-policy-extension-source
        user-interface-subagent-profile
        user-interface-subagent-route-policy
        user-interface-subagent-context
        user-interface-subagent-policy-extension)

;;; Boundary: Small user config lookup keeps the public API record-like.
;; MarlinResult <- MarlinInput Symbol MarlinInput
(def (user-interface-config-ref/default config slot-name default-value)
  (if (member slot-name (.all-slots config))
    (.ref config slot-name)
    default-value))

;;; Boundary: project-root is the public alias; workspace-root is receipt-facing.
;; String <- POOObject
(def (user-interface-workspace-root config)
  (user-interface-config-ref/default
   config
   'workspace-root
   (user-interface-config-ref/default
    config
    'project-root
    "user-interface-module-config")))

;;; Boundary: Root user config schema is upstream-maintained.
;; MarlinResult <- MarlinInput
(def UserInterfaceModuleConfig
  (marlin-module-interface
   "UserInterfaceModuleConfig"
   (.o surface: (marlin-string-constant "downstream-user-interface")
       entry: (marlin-string-constant "interface-workflow")
       layer: (marlin-string-optional))
   '((owner . "marlin") (surface . "user-interface-prefab"))))

;;; Boundary: Workspace defaults are prefab furniture, not downstream plumbing.
;; MarlinResult <- MarlinInput
(def UserInterfaceWorkspaceProfile
  (marlin-module-interface
   "UserInterfaceWorkspaceProfile"
   (.o workspace-root: (marlin-string-default "user-interface-module-config")
       interface-file: (marlin-string-default "interface.org")
       state-file: (marlin-string-default "state/worker-state.org"))
   '((owner . "marlin") (surface . "user-interface-prefab"))))

;;; Boundary: Agent defaults live in the furnished upstream pack.
;; MarlinResult <- MarlinInput
(def UserInterfaceAgentProfile
  (marlin-module-interface
   "UserInterfaceAgentProfile"
   (.o agent-scope: (marlin-string-default "user-interface-agent")
       agent-class: (marlin-string-default "customer-user-interface")
       model-profile: (marlin-string-default "interactive"))
   '((owner . "marlin") (surface . "user-interface-prefab"))))

;;; Boundary: Hook defaults only name existing Rust catalog handlers.
;; MarlinResult <- MarlinInput
(def UserInterfaceHookProfile
  (marlin-module-interface
   "UserInterfaceHookProfile"
   (.o hook-id: (marlin-string-default "runtime-catalog-user-interface-hook")
       hook-action: (marlin-string-default "register")
       hook-owner: (marlin-string-default "user-interface-worker"))
   '((owner . "marlin") (surface . "user-interface-prefab"))))

;;; Boundary: Continuation defaults are still Scheme POO intent.
;; MarlinResult <- MarlinInput
(def UserInterfaceLoopContinuationProfile
  (marlin-module-interface
   "UserInterfaceLoopContinuationProfile"
   (.o continuation-profile:
       (marlin-string-default "user-interface-loop-continuation"))
   '((owner . "marlin") (surface . "user-interface-prefab"))))

;;; Boundary: Root config users touch before moving into the furnished house.
;; MarlinResult <- MarlinInput
(def (user-interface-root-module config)
  (let ((surface-value
         (user-interface-config-ref/default
          config 'surface "downstream-user-interface"))
        (entry-value
         (user-interface-config-ref/default
          config 'entry "interface-workflow")))
    (marlinModules
     UserInterfaceModuleConfig
     (.o id: "user-interface-root-module"
         config:
         (.o surface: surface-value
             entry: entry-value)))))

;;; Boundary: Workspace room is generated from the small public config.
;; MarlinResult <- MarlinInput
(def (make-user-interface-workspace-module config)
  (let ((workspace-root-value (user-interface-workspace-root config))
        (interface-file-value
         (user-interface-config-ref/default
          config 'interface-file "interface.org"))
        (state-file-value
         (user-interface-config-ref/default
          config 'state-file "state/worker-state.org")))
    (marlinModules
     UserInterfaceWorkspaceProfile
     (.o id: "user-interface-workspace-module"
         config:
         (.o workspace-root: workspace-root-value
             interface-file: interface-file-value
             state-file: state-file-value)))))

;;; Boundary: Agent room is generated from the same public config.
;; MarlinResult <- MarlinInput
(def (make-user-interface-agent-module config)
  (let ((agent-scope-value
         (user-interface-config-ref/default
          config 'agent-scope "user-interface-agent"))
        (agent-class-value
         (user-interface-config-ref/default
          config 'agent-class "customer-user-interface"))
        (model-profile-value
         (user-interface-config-ref/default
          config 'model-profile "interactive")))
    (marlinModules
     UserInterfaceAgentProfile
     (.o id: "user-interface-agent-module"
         config:
         (.o agent-scope: agent-scope-value
             agent-class: agent-class-value
             model-profile: model-profile-value)))))

;;; Boundary: Hook room records catalog ids without creating handlers.
;; MarlinResult <- MarlinInput
(def (make-user-interface-hook-module config)
  (let ((hook-id-value
         (user-interface-config-ref/default
          config 'hook-id "runtime-catalog-user-interface-hook"))
        (hook-action-value
         (user-interface-config-ref/default config 'hook-action "register"))
        (hook-owner-value
         (user-interface-config-ref/default
          config 'hook-owner "user-interface-worker")))
    (marlinModules
     UserInterfaceHookProfile
     (.o id: "user-interface-hook-module"
         config:
         (.o hook-id: hook-id-value
             hook-action: hook-action-value
             hook-owner: hook-owner-value)))))

;;; Boundary: Runtime catalog action is named; Scheme creates no handlers.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-extension user-interface-extension
  "user-interface-worker-extension"
  (list "dynamic-hook-action" "customer-agent-policy" "high-order-matcher")
  (make-marlin-deck-runtime-condition-policy
   "user-interface-session"
   (list "root-agent" "user-interface-agent")
   (list "user-interface-worker-ready" "interface-state-open")
   (list "ui-memory" "worker-state-active")
   "customer-user-interface")
  (make-marlin-deck-runtime-high-order-matcher
   "user-interface-command"
   (lambda (_context _policy command agent-scope)
     (and (string=? command "codex user-interface workflow apply")
          (string=? agent-scope "user-interface-agent"))))
  (make-marlin-deck-runtime-register-hook-action
   "runtime-catalog-user-interface-hook"
   "runtime-catalog-user-interface-hook")
  '((owner . "marlin") (surface . "user-interface-prefab")))

;;; Boundary: Base module exports extension state and base options.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-user-module user-interface-base-module
  "user-interface-base-module"
  '()
  (list user-interface-extension)
  '()
  (list
   (make-marlin-deck-runtime-option-config
    "layer"
    "base"
    "user-interface-base-module"
    '((owner . "marlin")))
   (make-marlin-deck-runtime-option-config
    "surface"
    "downstream-user-interface"
    "user-interface-base-module"
    '((owner . "marlin"))))
  '((owner . "marlin")))

;;; Boundary: User-facing script entrypoint is prefab-owned Scheme source.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-script user-interface-worker-script
  "user-interface-worker-script"
  user-interface-extension
  "register"
  '((owner . "marlin") (entry . "interface-workflow"))
  (context)
  (.o kind: "user-interface-workflow-result.v1"
      command: (.get context command)
      agent-scope: (.get context agent-scope)
      workspace-root: (.get context workspace-root)
      has-interface-file: (file-exists? "interface.org")
      has-worker-state-file: (file-exists? "state/worker-state.org")
      extension-id:
      (.get (marlin-deck-runtime-script-extension user-interface-worker-script)
            id)))

;;; Boundary: Script module depends on base module and exports script state.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-user-module user-interface-script-module
  "user-interface-script-module"
  (list user-interface-base-module)
  '()
  (list user-interface-worker-script)
  (list
   (make-marlin-deck-runtime-option-config
    "layer"
    "script"
    "user-interface-script-module"
    '((owner . "marlin")))
   (make-marlin-deck-runtime-option-config
    "entry"
    "interface-workflow"
    "user-interface-script-module"
    '((owner . "marlin"))))
  '((owner . "marlin")))

;;; Boundary: Continuation node names a Rust executor catalog handle.
;; MarlinResult <- MarlinInput
(def user-interface-loop-continuation-node
  (make-marlin-deck-runtime-loop-node
   "policy"
   "gerbil.poo.policy"
   '(("source" . "poo")
     ("workspace" . "user-interface-module-config"))))

;;; Boundary: Prefab graph declaration is a regular POO value.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-loop-graph user-interface-loop-continuation-graph
  "user-interface-continuation-graph"
  (user-interface-loop-continuation-node)
  ())

;;; Boundary: Scheme compiles graph shape; Rust validates before execution.
;; MarlinResult <- MarlinInput
(def user-interface-loop-continuation-compiled-graph
  (marlin-deck-runtime-compile-loop-graph
   user-interface-loop-continuation-graph))

;;; Boundary: Base profile is intentionally terminal and safe by default.
;; MarlinResult <- MarlinInput
(defmarlin-graph-loop-continuation-profile
  user-interface-loop-continuation-base-profile
  "user-interface-loop-continuation"
  (make-marlin-graph-loop-continuation-stop-completed-action)
  '("poo_continuation=default_stop"))

;;; Boundary: POO extension overrides only action and diagnostics lazily.
;; MarlinResult <- MarlinInput
(def user-interface-loop-continuation-profile
  (.o (:: @ (list user-interface-loop-continuation-base-profile))
      action:
      (make-marlin-graph-loop-continuation-continue-with-graph-action
       user-interface-loop-continuation-compiled-graph)
      diagnostics:
      '("poo_continuation=continue"
        "workspace=user-interface-module-config")))

;;; Boundary: Projection is a typed POO object handed to Rust native APIs.
;; MarlinResult <- MarlinInput
(def user-interface-loop-continuation-projection
  (marlin-graph-loop-continuation-next-action
   user-interface-loop-continuation-profile))

;;; Boundary: Continuation module is generated from public config defaults.
;; MarlinResult <- MarlinInput
(def (make-user-interface-loop-continuation-module config)
  (let (continuation-profile-value
        (user-interface-config-ref/default
         config
         'continuation-profile
         "user-interface-loop-continuation"))
    (marlinModules
     UserInterfaceLoopContinuationProfile
     (.o id: "user-interface-loop-continuation-module"
         config:
         (.o continuation-profile: continuation-profile-value)))))

;;; Boundary: Source identity is owned by the upstream module system.
;; MarlinResult <- MarlinInput
(def user-interface-subagent-policy-extension-source
  ":modules/prefabs/user-interface#subagent")

;;; Boundary: The agent-authored profile becomes prefab furniture.
;; MarlinResult <- MarlinInput
(def user-interface-subagent-profile
  (.o id: "user-interface-review-subagent"
      agent-class: "customer-user-interface"
      lineage: '("root-agent"
                 "user-interface-agent"
                 "user-interface-review-subagent")
      spawn-command: "codex subagent spawn user-interface-review-subagent"))

;;; Boundary: Agent-authored policy stays a typed policy object.
;; MarlinResult <- MarlinInput
(def user-interface-subagent-route-policy
  (make-marlin-deck-runtime-model-route-policy
   "user-interface-subagent-route"
   "openai"
   "gpt-5.4"
   '("codex user-interface")
   '("user-interface-agent")
   "shared-context"
   "workspace-isolated"))

;;; Boundary: Test context proves the extension can match workspace state.
;; MarlinResult <- MarlinInput
(def user-interface-subagent-context
  (make-marlin-deck-runtime-strategy-context
   "user-interface-session"
   '("root-agent" "user-interface-agent")
   '("workspace-ready")
   '("interface.org")
   "customer-user-interface"))

;;; Boundary: Extension conditions remain Scheme policy objects.
;; MarlinResult <- MarlinInput
(def user-interface-subagent-condition
  (make-marlin-deck-runtime-condition-policy
   "user-interface-session"
   '("root-agent" "user-interface-agent")
   '("workspace-ready")
   '("interface.org")
   "customer-user-interface"))

;;; Boundary: Agent subagent launch policy is high-order matching.
;; MarlinResult <- MarlinInput
(def user-interface-subagent-matcher
  (make-marlin-deck-runtime-high-order-matcher
   "user-interface-subagent-command"
   (lambda (context policy command agent-scope)
     (and (string=? (.get context agent-class) "customer-user-interface")
          (string=? (.get policy provider) "openai")
          (string=? command "codex user-interface spawn-subagent")
          (string=? agent-scope "user-interface-agent")))))

;;; Boundary: The extension object combines subagent, hook, and policy slots.
;; MarlinResult <- MarlinInput
(defmarlin-policy-extension user-interface-subagent-policy-extension
  (source user-interface-subagent-policy-extension-source)
  (object
   (make-marlin-deck-runtime-subagent-policy-extension
    "user-interface-subagent-policy-extension"
    user-interface-subagent-profile
    user-interface-subagent-route-policy
    user-interface-subagent-condition
    user-interface-subagent-matcher
    (make-marlin-deck-runtime-register-hook-action
     "runtime-catalog-user-interface-hook"
     "runtime-catalog-user-interface-hook")
    '((owner . "marlin")
      (surface . "user-interface-prefab"))))
  (metadata '((owner . "marlin")
              (surface . "module-managed-policy-extension"))))

;;; Boundary: Public user API: move into the furnished user-interface house.
;; MarlinResult <- MarlinInput
(def (UserInterfaceWorkspace config)
  (user-interface-prefab-module-config
   (user-interface-root-module config)
   config))

;;; Boundary: Upstream owns module imports and extension composition.
;; MarlinResult <- MarlinInput
(def (user-interface-prefab-module-config user-config . maybe-config)
  (let (config
        (if (null? maybe-config)
          (.o)
          (car maybe-config)))
    (.o (:: @ (list user-config))
        (imports => marlin-imports-append
                 (marlin-imports
                  (marlin-import
                   ":modules/prefabs/user-interface#workspace"
                   (make-user-interface-workspace-module config))
                  (marlin-import
                   ":modules/prefabs/user-interface#agent"
                   (make-user-interface-agent-module config))
                  (marlin-import
                   ":modules/prefabs/user-interface#loop-continuation"
                   (make-user-interface-loop-continuation-module config))
                  (marlin-import
                   ":modules/prefabs/user-interface#script"
                   user-interface-script-module)
                  (marlin-import
                   ":modules/prefabs/user-interface#hooks"
                   (make-user-interface-hook-module config))))
        (extensions => marlin-extensions-append
                    (marlin-extensions
                     user-interface-subagent-policy-extension)))))

;;; Boundary: Catalog/eval/presentation are upstream plumbing.
;; MarlinResult <- MarlinInput
(def (UserInterfaceWorkspaceCatalog module-config)
  (marlinModuleCatalog module-config))

;;; Boundary: Allowed Rust hook ids stay explicit but upstream-owned.
;; MarlinResult <- MarlinInput
(def (UserInterfaceWorkspaceEvaluation module-config)
  (marlinEvalModules
   (UserInterfaceWorkspaceCatalog module-config)
   "user-interface-root-module"
   '("runtime-catalog-user-interface-hook")))

;;; Boundary: Presentation receipt is built from the same prefab defaults.
;; MarlinResult <- MarlinInput
(def (UserInterfaceWorkspaceSystemPresentation module-config)
  (marlinModuleSystemPresentation
   (UserInterfaceWorkspaceCatalog module-config)
   "user-interface-root-module"
   '("runtime-catalog-user-interface-hook")))

;;; Boundary: Workflow materializes runtime projections for scripts/extensions.
;; MarlinResult <- MarlinInput
(def (UserInterfaceWorkspaceWorkflow module-config)
  (marlin-module-workflow
   module-config
   '("runtime-catalog-user-interface-hook")))

;;; Boundary: The subagent extension is policy furniture inside the prefab.
;; MarlinResult <- MarlinInput
(def user-interface-subagent-policy-object
  (marlinPolicyObject
   "subagent-policy-extension"
   "user-interface-subagent-policy-extension"
   user-interface-subagent-policy-extension
   '((owner . "marlin") (surface . "user-interface-prefab-object"))))

;;; Boundary: Continuation policy stays a POO object before Rust projection.
;; MarlinResult <- MarlinInput
(def user-interface-continuation-policy-object
  (marlinPolicyObject
   "continuation-profile"
   "user-interface-loop-continuation"
   user-interface-loop-continuation-profile
   '((owner . "marlin") (surface . "user-interface-prefab-object"))))

;;; Boundary: Hook selector object only names an existing Rust catalog handler.
;; MarlinResult <- MarlinInput
(def user-interface-hook-policy-object
  (marlinPolicyObject
   "hook-selection-policy"
   "runtime-catalog-user-interface-hook"
   (.o hook-id: "runtime-catalog-user-interface-hook"
       action: "register")
   '((owner . "marlin") (surface . "user-interface-prefab-object"))))

;;; Boundary: Added memory trigger is normal policy furniture.
;; MarlinResult <- MarlinInput
(def user-interface-memory-policy-object
  (marlinMemoryTriggerPolicy
   "user-interface-memory-trigger"
   (.o trigger: "context-pressure"
       action: "compact")
   '((owner . "marlin") (surface . "user-interface-prefab-object"))))

;;; Boundary: Replacement object keeps typed projection separate from merge.
;; MarlinResult <- MarlinInput
(def user-interface-continuation-projection-object
  (marlinPolicyObject
   "continuation-profile"
   "user-interface-continuation-projection"
   user-interface-loop-continuation-projection
   '((owner . "marlin") (surface . "user-interface-prefab-object"))))

;;; Boundary: User-facing prefab pack composes modules plus policy objects.
;; MarlinResult <- MarlinInput
(def (UserInterfacePolicyPack module-config)
  (let* ((default-pack (marlinDefaultPolicyPack module-config))
         (default-policy-objects
          (.get default-pack default-policy-objects))
         (default-allowed-hook-ids
          (.get default-pack allowed-hook-ids)))
    (marlinPolicyPack
     (.o id: "user-interface-prefab-pack"
         module: module-config
         policy-objects:
         (append
          default-policy-objects
          (list user-interface-subagent-policy-object
                user-interface-continuation-policy-object
                user-interface-hook-policy-object))
         object-operations:
         (list
          (marlin-add-object
           user-interface-memory-policy-object
           "add memory trigger furniture")
          (marlin-remove-object
           "hook-selection-policy"
           "runtime-catalog-user-interface-hook"
           "Rust catalog owns hook handlers")
          (marlin-disable-object
           "subagent-policy-extension"
           "user-interface-subagent-policy-extension"
           "disabled by downstream object surgery")
          (marlin-replace-object
           "continuation-profile"
           "user-interface-loop-continuation"
           user-interface-continuation-projection-object
           "replace profile object with typed projection object"))
         allowed-hook-ids:
         (append
          default-allowed-hook-ids
          '("runtime-catalog-user-interface-hook"))
         metadata:
         '((owner . "marlin") (surface . "user-interface-prefab-pack"))))))

;;; Boundary: Pack catalog is the prefab collection entrypoint.
;; MarlinResult <- MarlinInput
(def (UserInterfacePolicyPackCatalog policy-pack)
  (marlinPackCatalog policy-pack))

;;; Boundary: Presentation gives Rust/debug tools a scalar prefab receipt.
;; MarlinResult <- MarlinInput
(def (UserInterfacePolicyPackPresentation policy-pack)
  (marlinPolicyPackPresentation policy-pack))
