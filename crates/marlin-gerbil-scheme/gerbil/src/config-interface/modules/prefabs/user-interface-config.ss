;;; -*- Gerbil -*-
;;; Boundary: User-interface prefab config adapters maintained by upstream Marlin.

package: config-interface/modules/prefabs

(import :gerbil/gambit
        (only-in :clan/poo/object .all-slots .o .ref)
        (only-in :poo-flow/src/module-system/facade
                 poo-flow-module-interface
                 poo-flow-string-constant
                 poo-flow-string-default
                 poo-flow-string-optional
                 poo-flow-user-module-bundles->modules
                 poo-flow-user-module-selection-key
                 poo-flow-user-module-selection-flag-entry)
        (only-in :poo-flow/src/module-system/root-profile
                 pooFlowRootProfile
                 pooFlowRootModules
                 pooFlowRootSettingKeys
                 pooFlowRootConfig)
        (only-in :std/sugar find))

(export user-interface-config-ref/default
        user-interface-contract-field
        user-interface-workspace-root
        user-interface-selection-ref/default
        UserInterfaceRootConfig
        UserInterfaceRootSelections
        UserInterfaceRootSelectionKeys
        UserInterfaceRootSelection
        UserInterfaceModuleBundleSelections
        UserInterfaceModuleBundleSelection
        UserInterfaceModuleBundleFlag
        UserInterfaceModuleBundleConfig
        UserInterfaceWorkspaceConfig
        UserInterfaceWorkspaceProfile
        UserInterfaceAgentProfile
        UserInterfaceHookProfile
        UserInterfaceLoopContinuationProfile)

;;; Boundary: Small user config lookup keeps the public API record-like.
;; : (-> POOObject Symbol Value Value)
(def (user-interface-config-ref/default config slot-name default-value)
  (if (member slot-name (.all-slots config))
    (.ref config slot-name)
    default-value))

;; : (-> Alist Symbol Value)
(def (user-interface-contract-field contract key)
  (let (entry (assq key contract))
    (and entry (cdr entry))))

;;; Boundary: project-root is the public alias; workspace-root is receipt-facing.
;; : (-> POOObject String)
(def (user-interface-workspace-root config)
  (user-interface-config-ref/default
   config
   'workspace-root
   (user-interface-config-ref/default
    config
    'project-root
    "user-interface-workspace")))

;;; Boundary: POPflow owns selection construction; Marlin only adapts flags
;;; into the furnished workspace config consumed by the prefab.
;; : (-> PooUserModuleSelection Symbol Value Value)
(def (user-interface-selection-ref/default selection key default-value)
  (let (entry (poo-flow-user-module-selection-flag-entry selection key))
    (if entry (cdr entry) default-value)))

;;; Boundary: Root config expansion belongs to POPflow. Marlin only exposes
;;; prefab-shaped bridge helpers so downstream examples avoid local plumbing.
;; : (-> (List PooUserModuleSelection) POOObject)
(def (UserInterfaceRootConfig module-bundles)
  (pooFlowRootConfig module-bundles))

;; : (-> (List PooUserModuleSelection) (List PooUserModuleSelection))
(def (UserInterfaceRootSelections module-bundles)
  (pooFlowRootModules
   (pooFlowRootProfile module-bundles)))

;; : (-> (List PooUserModuleSelection) (List Symbol))
(def (UserInterfaceRootSelectionKeys module-bundles)
  (pooFlowRootSettingKeys
   (pooFlowRootProfile module-bundles)))

;; : (-> (List PooUserModuleSelection) Symbol PooUserModuleSelection)
(def (UserInterfaceRootSelection module-bundles selection-key)
  (find (lambda (selection)
          (equal? (poo-flow-user-module-selection-key selection)
                  selection-key))
        (UserInterfaceRootSelections module-bundles)))

;;; Boundary: A POPflow module bundle can expand to one or more selections.
;;; The user-interface prefab expects exactly the first configured workspace
;;; selection; POPflow still owns the bundle expansion mechanics.
;; : (-> PooUserModuleBundle (List PooUserModuleSelection))
(def (UserInterfaceModuleBundleSelections module-bundle)
  (poo-flow-user-module-bundles->modules (list module-bundle)))

;; : (-> PooUserModuleBundle PooUserModuleSelection)
(def (UserInterfaceModuleBundleSelection module-bundle)
  (let (selections (UserInterfaceModuleBundleSelections module-bundle))
    (if (null? selections)
      (error "empty POPflow module bundle for Marlin user-interface prefab")
      (car selections))))

;; : (-> PooUserModuleBundle Symbol Value)
(def (UserInterfaceModuleBundleFlag module-bundle flag)
  (let* ((selection (UserInterfaceModuleBundleSelection module-bundle))
         (entry (poo-flow-user-module-selection-flag-entry selection flag)))
    (and entry (cdr entry))))

;; : (-> PooUserModuleBundle POOObject)
(def (UserInterfaceModuleBundleConfig module-bundle)
  (or (UserInterfaceModuleBundleFlag module-bundle ':config)
      (error "missing :config flag in POPflow module bundle"
             (UserInterfaceModuleBundleSelection module-bundle))))

;;; Boundary: Root user config schema is upstream-maintained.
;; : InterfaceDescriptor
(def UserInterfaceWorkspaceConfig
  (poo-flow-module-interface
   "UserInterfaceWorkspaceConfig"
   (.o surface: (poo-flow-string-constant "downstream-user-interface")
       entry: (poo-flow-string-constant "interface-workflow")
       layer: (poo-flow-string-optional))
   '((owner . "marlin") (surface . "user-interface-prefab"))))

;;; Boundary: Workspace defaults are prefab furniture, not downstream plumbing.
;; : InterfaceDescriptor
(def UserInterfaceWorkspaceProfile
  (poo-flow-module-interface
   "UserInterfaceWorkspaceProfile"
   (.o workspace-root: (poo-flow-string-default "user-interface-workspace")
       interface-file: (poo-flow-string-default "interface.org")
       state-file: (poo-flow-string-default "state/worker-state.org"))
   '((owner . "marlin") (surface . "user-interface-prefab"))))

;;; Boundary: Agent defaults live in the furnished upstream pack.
;; : InterfaceDescriptor
(def UserInterfaceAgentProfile
  (poo-flow-module-interface
   "UserInterfaceAgentProfile"
   (.o agent-scope: (poo-flow-string-default "user-interface-agent")
       agent-class: (poo-flow-string-default "customer-user-interface")
       model-profile: (poo-flow-string-default "interactive"))
   '((owner . "marlin") (surface . "user-interface-prefab"))))

;;; Boundary: Hook defaults only name existing Rust catalog handlers.
;; : InterfaceDescriptor
(def UserInterfaceHookProfile
  (poo-flow-module-interface
   "UserInterfaceHookProfile"
   (.o hook-id: (poo-flow-string-default "runtime-catalog-user-interface-hook")
       hook-action: (poo-flow-string-default "register")
       hook-owner: (poo-flow-string-default "user-interface-worker"))
   '((owner . "marlin") (surface . "user-interface-prefab"))))

;;; Boundary: Continuation defaults are still Scheme POO intent.
;; : InterfaceDescriptor
(def UserInterfaceLoopContinuationProfile
  (poo-flow-module-interface
   "UserInterfaceLoopContinuationProfile"
   (.o continuation-profile:
       (poo-flow-string-default "user-interface-loop-continuation"))
   '((owner . "marlin") (surface . "user-interface-prefab"))))
