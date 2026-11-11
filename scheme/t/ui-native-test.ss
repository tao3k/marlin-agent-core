;;; Run the workbench contract through the SDK's in-process GxTest entrypoint.
(import :std/test :gerbil/expander
        (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports)
        (only-in "ui-workbench-test" ui-workbench-verified))
(export ui-native-test)

(def ui-native-test
  (test-suite "Marlin UI workbench"
    (test-case "the selected Scheme UI contract completed"
      (check ui-workbench-verified => #t))
    (test-case "standard module import retains the selected UI scope"
      (let ((expected (.get marlin-agent-core.init.exports Marlin ui)))
        (parameterize ((current-expander-context (make-top-context)))
          (eval-syntax '(import :gerbil/core))
          (eval-syntax '(import (only-in :marlin-agent-core/init marlin-agent-core.init.exports)))
          (let (selected (.get (eval 'marlin-agent-core.init.exports) Marlin ui))
            (check (eq? selected expected) => #t))
          (eval-syntax '(import (only-in :marlin-agent-core/c-abi/ui
                                        marlin-ui-session-open marlin-ui-session-project)))
          (let* ((publication (eval 'marlin-agent-core.init.exports))
                 (open (eval 'marlin-ui-session-open))
                 (project (eval 'marlin-ui-session-project))
                 (state (open (vector publication #("model" "session" #() #()) #())))
                 (view (project state)))
            (check-equal? (vector-ref view 0) "model")
            (check-equal? (vector-ref view 3) 'composer)))))))
