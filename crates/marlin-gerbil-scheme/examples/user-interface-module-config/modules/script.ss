;;; -*- Gerbil -*-
;;; Boundary: Downstream example script module imports the base user module.

(import :clan/poo/object
        :marlin/deck-runtime-script
        :marlin/deck-runtime-user-module
        :marlin/deck-runtime-user-option
        "base")

(export user-interface-script-module
        user-interface-worker-script)

;;; Boundary: User-facing script entrypoint is real Scheme source in the example workspace.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
(defmarlin-deck-runtime-script user-interface-worker-script
  "user-interface-worker-script"
  user-interface-extension
  "register"
  '((owner . "user-interface-worker") (entry . "interface-workflow"))
  (context)
  (.o kind: "user-interface-workflow-result.v1"
      command: (.get context command)
      agent-scope: (.get context agent-scope)
      workspace-root: (.get context workspace-root)
      has-interface-file: (file-exists? "interface.org")
      has-worker-state-file: (file-exists? "state/worker-state.org")
      extension-id:
      (.get (marlin-deck-runtime-script-extension user-interface-worker-script) id)))

;;; Boundary: Script module depends on base module and exports script state.
;; UserInterfaceWorkflowResult <- UserInterfaceWorkflowContext
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
    '((owner . "user-interface-worker")))
   (make-marlin-deck-runtime-option-config
    "entry"
    "interface-workflow"
    "user-interface-script-module"
    '((owner . "user-interface-worker"))))
  '((owner . "user-interface-worker")))
