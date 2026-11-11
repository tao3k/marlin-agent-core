;;; This value describes the Module only; it does not assert ABI readiness.
(import (only-in :clan/poo/object .o))
(export make-c-abi-operations)
(def (make-c-abi-operations ui-value context-value deck-value)
  (.o identity: 'marlin/c-abi ui: ui-value context: context-value deck-runtime: deck-value))
