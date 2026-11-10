;;; Operations provided by the Policy module.

(import (only-in :clan/poo/object .o)
        "./func.ss")

(export PolicyOperations)

(def PolicyOperations
  (.o select-model-route: marlin-deck-runtime-select-model-route-policy/strategy
      project-routing: marlin-agent-policy-routing-project-poo-policy
      compile-loop-graph: marlin-deck-runtime-compile-loop-graph
      next-continuation: marlin-graph-loop-continuation-next-action
      project-script: marlin-deck-runtime-script-native-projection))
