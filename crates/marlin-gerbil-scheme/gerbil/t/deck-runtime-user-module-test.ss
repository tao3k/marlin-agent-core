;;; -*- Gerbil -*-
;;; Boundary: Module tests NixOS-style user module composition for Gerbil POO extensions.

(import :clan/poo/object
        :marlin/deck-runtime
        :marlin/deck-runtime-condition-policy
        :marlin/deck-runtime-dynamic-hook
        :marlin/deck-runtime-extension-catalog
        :marlin/deck-runtime-extension-template
        :marlin/deck-runtime-matcher
        :marlin/deck-runtime-script
        :marlin/deck-runtime-strategy-context
        :marlin/deck-runtime-user-module
        :marlin/deck-runtime-user-option
        :std/test)

;;; Boundary: Fixture context stays in the Scheme POO extension plane.
;; MarlinResult <- MarlinInput
(def user-module-context
  (make-marlin-deck-runtime-strategy-context
   "session-user-module"
   '("root-agent" "user-module-agent")
   '("workspace-ready")
   '("module-memory")
   "customer-user-module"))

;;; Boundary: Fixture policy remains a typed POO route policy.
;; MarlinResult <- MarlinInput
(def user-module-policy
  (make-marlin-deck-runtime-model-route-policy
   "user-module-policy"
   "openai"
   "gpt-5.4"
   '("codex")
   '("user-module-agent")
   "forked-context"
   "workspace-isolated"))

;;; Boundary: Condition object is provided by a user module.
;; MarlinResult <- MarlinInput
(def user-module-condition
  (make-marlin-deck-runtime-condition-policy
   "session-user-module"
   '("root-agent" "user-module-agent")
   '("workspace-ready")
   '("module-memory")
   "customer-user-module"))

;;; Boundary: High-order matcher object is provided by a user module.
;; MarlinResult <- MarlinInput
(def user-module-matcher
  (make-marlin-deck-runtime-high-order-matcher
   "user-module-command"
   (lambda (_context _policy command agent-scope)
     (and (string=? command "codex user-module apply")
          (string=? agent-scope "user-module-agent")))))

;;; Boundary: Extension object is declared through the upstream macro plane.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-extension user-module-extension
  "user-module-extension"
  '("dynamic-hook-action" "high-order-matcher")
  user-module-condition
  user-module-matcher
  (make-marlin-deck-runtime-register-hook-action
   "runtime-catalog-user-module-hook"
   "runtime-catalog-user-module-hook")
  '((owner . "user-module-base")))

;;; Boundary: Script object is declared through the upstream macro plane.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-script user-module-script
  "user-module-script"
  user-module-extension
  "register"
  '((owner . "user-module-script"))
  (context)
  (.o kind: "user-module-script-result.v1"
      command: (.get context command)
      agent-scope: (.get context agent-scope)
      extension-id:
      (.get (marlin-deck-runtime-script-extension user-module-script) id)))

;;; Boundary: Base module exports only extension state.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-user-module user-module-base
  "user-module-base"
  '()
  (list user-module-extension)
  '()
  (list
   (make-marlin-deck-runtime-option-config
    "state"
    "base"
    "user-module-base"
    '((owner . "base-module"))))
  '((owner . "base-module")))

;;; Boundary: Script module imports base module like a NixOS module import.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-user-module user-module-script-module
  "user-module-script-module"
  (list user-module-base)
  '()
  (list user-module-script)
  (list
   (make-marlin-deck-runtime-option-config
    "entry"
    "script"
    "user-module-script-module"
    '((owner . "script-module"))))
  '((owner . "script-module")))

;;; Boundary: Root module imports downstream modules without recreating their POO objects.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-user-module user-module-root
  "user-module-root"
  (list user-module-script-module user-module-base)
  '()
  '()
  (list
   (make-marlin-deck-runtime-option-config
    "surface"
    "root"
    "user-module-root"
    '((owner . "root-module"))))
  '((owner . "root-module")))

;;; Boundary: Evaluation materializes the import graph into typed Scheme values.
;; MarlinResult <- MarlinInput
(def user-module-evaluation
  (marlin-deck-runtime-user-module-evaluate user-module-root))

;;; Boundary: Module id projection gives deterministic imports-first ordering.
;; MarlinResult <- MarlinInput
(def (user-module-evaluation-module-ids)
  (.get user-module-evaluation module-ids))

;;; Boundary: Script id projection proves scripts are collected from imports.
;; MarlinResult <- MarlinInput
(def (user-module-evaluation-script-ids)
  (map (lambda (script) (.get script id))
       (.get user-module-evaluation scripts)))

;;; Boundary: Extension id projection proves duplicate imports are deduplicated.
;; MarlinResult <- MarlinInput
(def (user-module-evaluation-extension-ids)
  (map (lambda (extension) (.get extension id))
       (.get user-module-evaluation extensions)))

;;; Boundary: Catalog projection stays Scheme-built and Rust-consumable.
;; MarlinResult <- MarlinInput
(def user-module-catalog
  (marlin-deck-runtime-user-module-extension-catalog
   user-module-evaluation
   '("runtime-catalog-user-module-hook")))

(check (.get user-module-evaluation kind)
       => marlin-deck-runtime-user-module-evaluation-kind)
(check (user-module-evaluation-module-ids)
       => '("user-module-base" "user-module-script-module" "user-module-root"))
(check (user-module-evaluation-extension-ids)
       => '("user-module-extension"))
(check (user-module-evaluation-script-ids)
       => '("user-module-script"))
(check (length (.get user-module-evaluation options)) => 3)
(check (map (lambda (option) (.get option id))
            (.get user-module-evaluation options))
       => '("surface" "entry" "state"))
(check (.get
        (marlin-deck-runtime-extension-catalog-find
         user-module-catalog
         "user-module-extension")
        id)
       => "user-module-extension")
(check (.get
        (marlin-deck-runtime-extension-catalog-select
         user-module-catalog
         user-module-context
         user-module-policy
         "codex user-module apply"
         "user-module-agent")
        id)
       => "user-module-extension")
(check (.get
        (car
         (marlin-deck-runtime-user-module-script-interface-receipts
          user-module-evaluation))
        script-id)
       => "user-module-script")
(check (.get
        (marlin-deck-runtime-user-module-run-script
         user-module-evaluation
         "user-module-script"
         (.o command: "codex user-module apply"
             agent-scope: "user-module-agent"))
        extension-id)
       => "user-module-extension")
