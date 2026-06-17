#!/usr/bin/env gxi
;;; -*- Gerbil -*-
;;; Build script for the Marlin Gerbil runtime assets.

(import :std/build-script)

(defrules defmarlin-runtime-build-script ()
  ((_ extra-target ...)
   (defbuild-script
     '((gsc: "src/marlin/deck-runtime-native")
       (ssi: "src/marlin/deck-runtime-native")
       (gsc: "src/marlin/agent-policy-routing-native")
       (ssi: "src/marlin/agent-policy-routing-native")
       "src/marlin/protocol-types"
       "src/marlin/protocol"
       "src/marlin/request"
       "src/marlin/parser"
       "src/marlin/adapter"
       "src/marlin/deck-runtime"
       "src/marlin/deck-runtime-native-projection"
       "src/marlin/graph-loop-continuation-native-projection"
       "src/marlin/agent-policy-routing-native-projection"
       "src/marlin/deck-runtime-compiled-policy"
       "src/marlin/deck-runtime-compiled-policy-sample"
       "src/marlin/deck-runtime-strategy-context"
       "src/marlin/deck-runtime-condition-policy"
       "src/marlin/deck-runtime-dynamic-hook"
       "src/marlin/deck-runtime-matcher"
       "src/marlin/deck-runtime-policy-engine"
       "src/marlin/deck-runtime-agent-policy"
       "src/marlin/deck-runtime-extension"
       "src/marlin/deck-runtime-extension-safety"
       "src/marlin/deck-runtime-extension-catalog"
       "src/marlin/deck-runtime-extension-receipt"
       "src/marlin/deck-runtime-extension-template"
       "src/marlin/deck-runtime-script"
       "src/marlin/deck-runtime-loop-graph"
       "src/marlin/deck-runtime-user-option"
       "src/marlin/deck-runtime-user-module"
       "src/modules/kinds"
       "src/modules/core"
       "src/modules/policy-extension"
       "src/modules/policy-module"
       "src/modules/policy-object"
       "src/modules/workspace-policy"
       "src/modules/session-policy"
       "src/modules/agent-policy"
       "src/modules/hook-selection-policy"
       "src/modules/model-route-policy"
       "src/modules/continuation-profile-policy"
       "src/modules/human-review-policy"
       "src/modules/evidence-policy"
       "src/modules/failure-policy"
       "src/modules/memory-policy"
       "src/modules/catalog-projection-policy"
       "src/modules/evaluation"
       "src/modules/policy-pack"
       "src/modules/lib"
       "src/modules/prefabs/default-policy"
       "src/modules/prefabs/user-interface"
       "src/modules/prefabs/user-interface-delivery"
       "src/marlin/deck-runtime-strategy"
       extra-target ...))))

(defmarlin-runtime-build-script)
