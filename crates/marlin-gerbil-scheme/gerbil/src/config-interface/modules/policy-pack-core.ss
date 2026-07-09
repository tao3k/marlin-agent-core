;;; -*- Gerbil -*-
;;; Engineering note: This core stays limited to pack construction and catalog
;;; lookup because module evaluation and Rust validation have separate receipts.
;;; Keep aggregation helpers expression-based so pack surgery remains auditable
;;; without expanding another broad generated object surface here.
package: config-interface/modules

(import (only-in :clan/poo/object .get .o .ref)
        (only-in :poo-flow/src/module-system/facade
                 poo-flow-module-object-has-slot?
                 poo-flow-module-object-ref/default
                 poo-flow-module-name
                 poo-flow-module-system-owner
                 poo-flow-scheme-owner)
        :config-interface/modules/kinds
        :config-interface/modules/policy-object
        :config-interface/modules/policy-pack-support
        :config-interface/modules/workspace-policy
        :config-interface/modules/session-policy
        :config-interface/modules/agent-policy
        :config-interface/modules/hook-selection-policy
        :config-interface/modules/model-route-policy
        :config-interface/modules/continuation-profile-policy
        :config-interface/modules/human-review-policy
        :config-interface/modules/evidence-policy
        :config-interface/modules/failure-policy
        :config-interface/modules/memory-policy
        :config-interface/modules/domain-policy
        :config-interface/modules/catalog-projection-policy)

(export marlinPolicyPack
        defmarlin-policy-pack
        marlinDefaultPolicyPack
        marlinPackCatalog
        marlin-pack-catalog-find
        marlin-pack-catalog-root
        marlinPolicyPackInventory
        marlin-policy-pack-root-module-kind
        marlin-policy-pack-module-evaluation-receipt-kind)

;;; Boundary: Policy packs build catalog metadata without importing evalModules.
;; marlin-policy-pack-module-catalog
;;   : (-> PooFlowModule ModuleCatalog)
(def (marlin-policy-pack-module-catalog module-value)
  (.o kind: marlin-module-catalog-kind
      modules:
      (map (lambda (catalog-module-value)
             catalog-module-value)
           (list module-value))))

;;; Boundary: Pack presentations summarize module metadata without evalModules.
;; marlin-policy-pack-root-module-kind
;;   : (-> PolicyPack (U String #f))
(def (marlin-policy-pack-root-module-kind policy-pack)
  (let (module-value (.get policy-pack module))
    (if module-value
      (.get module-value kind)
      #f)))

;;; Boundary: policy packs accept raw poo-flow module descriptors.
;; marlin-policy-pack-module-id
;;   : (-> (U PooFlowModule #f) (U String #f))
(def (marlin-policy-pack-module-id module-value)
  (if module-value
    (let (id-candidates
          (filter (lambda (candidate-value)
                    candidate-value)
                  (list
                   (if (poo-flow-module-object-has-slot? module-value 'id)
                     (.get module-value id)
                     #f)
                   (poo-flow-module-name module-value))))
      (if (pair? id-candidates)
        (car id-candidates)
        #f))
    #f))

;;; Boundary: Policy pack projection keeps runtime evaluation out of facade load.
;; marlin-policy-pack-module-evaluation-receipt-kind
;;   : String
(def marlin-policy-pack-module-evaluation-receipt-kind
  "marlin.config-interface.policy-pack.module-evaluation-receipt.v1")

;;; Boundary: Policy packs are upstream prefab bundles over POO modules.
;; marlinPolicyPack
;;   : (-> PolicyPackConfig PolicyPack)
(def (marlinPolicyPack pack-config)
  (let* ((module-value
          (poo-flow-module-object-ref/default pack-config 'module #f))
         (catalog-value
          (poo-flow-module-object-ref/default
           pack-config
           'catalog
           (if module-value
             (marlin-policy-pack-module-catalog module-value)
             #f)))
         (root-module-id-value
         (poo-flow-module-object-ref/default
           pack-config
           'root-module-id
           (if module-value
             (marlin-policy-pack-module-id module-value)
             #f)))
         (default-policy-objects-value
          (poo-flow-module-object-ref/default
           pack-config
           'policy-objects
           '()))
         (object-operations-value
          (poo-flow-module-object-ref/default
           pack-config
           'object-operations
           '()))
         (operation-result
          (marlin-policy-pack-apply-operations
           default-policy-objects-value
           object-operations-value))
         (surgery-receipts-value
          (.get operation-result surgery-receipts))
         (object-operation-count-value
          (foldl +
                 0
                 (list
                  (.get operation-result add-operation-count)
                  (.get operation-result remove-operation-count)
                  (.get operation-result disable-operation-count)
                  (.get operation-result replace-operation-count)))))
    (marlin-policy-object<-alist
     (list
      (cons 'kind marlin-policy-pack-kind)
      (cons 'id
            (poo-flow-module-object-ref/default
             pack-config
             'id
             "anonymous-marlin-policy-pack"))
      (cons 'module module-value)
      (cons 'catalog catalog-value)
      (cons 'root-module-id root-module-id-value)
      (cons 'allowed-hook-ids
            (poo-flow-module-object-ref/default
             pack-config
             'allowed-hook-ids
             '()))
      (cons 'default-policy-objects default-policy-objects-value)
      (cons 'policy-objects (.get operation-result policy-objects))
      (cons 'object-operations object-operations-value)
      (cons 'object-surgery-receipts surgery-receipts-value)
      (cons 'policy-object-count
            (length (.get operation-result policy-objects)))
      (cons 'object-operation-count object-operation-count-value)
      (cons 'object-surgery-receipt-count object-operation-count-value)
      (cons 'disabled-policy-object-count
            (marlin-policy-disabled-object-count
             (.get operation-result policy-objects)))
      (cons 'add-operation-count (.get operation-result add-operation-count))
      (cons 'remove-operation-count (.get operation-result remove-operation-count))
      (cons 'disable-operation-count (.get operation-result disable-operation-count))
      (cons 'replace-operation-count (.get operation-result replace-operation-count))
      (cons 'matched-surgery-receipt-count
            (.get operation-result matched-surgery-receipt-count))
      (cons 'conflict-surgery-receipt-count
            (.get operation-result conflict-surgery-receipt-count))
      (cons 'duplicate-object-conflict-count
            (marlin-policy-surgery-conflict-reason-count
             surgery-receipts-value
             "duplicate-object"))
      (cons 'missing-target-conflict-count
            (marlin-policy-surgery-conflict-reason-count
             surgery-receipts-value
             "missing-target"))
      (cons 'disabled-target-conflict-count
            (marlin-policy-surgery-conflict-reason-count
             surgery-receipts-value
             "disabled-target"))
      (cons 'invalid-replacement-conflict-count
            (marlin-policy-surgery-conflict-reason-count
             surgery-receipts-value
             "invalid-replacement"))
      (cons 'metadata
            (poo-flow-module-object-ref/default
             pack-config
             'metadata
             '()))
      (cons 'owner poo-flow-scheme-owner)
      (cons 'runtime-owner "rust")
      (cons 'rust-parses-scheme-source #f)
      (cons 'rust-handler-manufactured #f)))))

;;; Boundary: Level-1 prefab API exposes object surgery without plumbing.
;; defmarlin-policy-pack
;;   : (-> Syntax PolicyPackBinding)
;;   | doc m%
;;       `defmarlin-policy-pack` expands declarative prefab policy pack data
;;       into a binding backed by `marlinPolicyPack`.
;;
;;       # Examples
;;       ```scheme
;;       (defmarlin-policy-pack demo (id "demo") (module mod)
;;         (policy-objects obj) (object-operations))
;;       ;; => demo
;;       ```
;;     %
(defrules defmarlin-policy-pack ()
  ((_ binding
      (id pack-id)
      (module module-value)
      (policy-objects object-value ...)
      (object-operations operation-value ...)
      (allowed-hook-ids allowed-hook-id-value ...)
      (metadata metadata-value))
   (def binding
     (marlinPolicyPack
      (.o id: pack-id
          module: module-value
          policy-objects: (list object-value ...)
          object-operations: (list operation-value ...)
          allowed-hook-ids: (list allowed-hook-id-value ...)
          metadata: metadata-value))))
  ((_ binding
      (id pack-id)
      (module module-value)
      (policy-objects object-value ...)
      (object-operations operation-value ...))
   (def binding
     (marlinPolicyPack
      (.o id: pack-id
          module: module-value
          policy-objects: (list object-value ...)
          object-operations: (list operation-value ...)
          allowed-hook-ids: '()
          metadata: '())))))

;;; Boundary: Default pack is a bootstrap catalog member, not a runtime eval.
;; marlinDefaultPolicyPack
;;   : PolicyPack
(def marlinDefaultPolicyPack
  (marlinPolicyPack
   (.o id: "marlin/default-policy-pack"
       module: #f
       policy-objects: '()
       object-operations: '()
       allowed-hook-ids: '()
       metadata: '((source . "config-interface/bootstrap")))))

;;; Boundary: Pack catalogs keep prefab bundles first-class.
;; marlinPackCatalog
;;   : (-> PolicyPack ... PackCatalog)
(def (marlinPackCatalog . pack-values)
  (.o kind: marlin-pack-catalog-kind
      packs:
      (map (lambda (pack-value)
             pack-value)
           pack-values)))

;;; Boundary: Inventory keeps default pack discovery explicit.
;; marlinPolicyPackInventory
;;   : PackCatalog
(def marlinPolicyPackInventory
  (marlinPackCatalog marlinDefaultPolicyPack))

;;; Boundary: Pack lookup is explicit and deterministic.
;; marlin-pack-catalog-find
;;   : (-> PackCatalog String (U PolicyPack #f))
(def (marlin-pack-catalog-find catalog pack-id-value)
  (let (matches
        (filter (lambda (pack)
                  (string=? (.get pack id) pack-id-value))
                (.get catalog packs)))
    (if (pair? matches)
      (car matches)
      #f)))

;;; Boundary: A missing pack id means the first catalog pack is the root.
;; marlin-pack-catalog-root
;;   : (-> PackCatalog (U String #f) PolicyPack)
(def (marlin-pack-catalog-root catalog pack-id-value)
  (cond
   (pack-id-value
    (or (marlin-pack-catalog-find catalog pack-id-value)
        (error "marlin policy pack root not found" pack-id-value)))
   ((pair? (.get catalog packs))
    (car (.get catalog packs)))
   (else
    (error "marlin policy pack catalog is empty"))))
