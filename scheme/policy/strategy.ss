;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; Facade for the modular Scheme-owned Deck runtime policy plane.

(import "./agent-routes.ss"
        "./strategy-evaluation.ss"
        "./strategy-facts.ss")

(export marlin-deck-runtime-strategy-rule-kind
        marlin-deck-runtime-strategy-facts-kind
        marlin-deck-runtime-strategy-selection-kind
        marlin-deck-runtime-dynamic-strategy-rule-kind
        marlin-deck-runtime-agent-policy-template-kind
        marlin-deck-runtime-strategy-policy-receipt-kind
        marlin-deck-runtime-strategy-capability-names
        make-marlin-deck-runtime-strategy-facts
        make-marlin-deck-runtime-strategy-rule
        make-marlin-deck-runtime-dynamic-strategy-rule
        make-marlin-deck-runtime-agent-policy-template
        defmarlin-deck-runtime-strategy-rule
        defmarlin-deck-runtime-agent-policy-template
        defmarlin-deck-runtime-agent-policy-template-set
        marlin-deck-runtime-agent-policy-template-select
        marlin-deck-runtime-agent-policy-templates->dynamic-rules
        marlin-deck-runtime-strategy-rule-match?
        marlin-deck-runtime-dynamic-strategy-rule-match?
        marlin-deck-runtime-select-model-route-policy/strategy
        marlin-deck-runtime-select-dynamic-strategy-rule
        marlin-deck-runtime-agent-policy-template->dynamic-rule
        marlin-deck-runtime-dynamic-strategy-rule-action-selection
        marlin-deck-runtime-dynamic-strategy-rule-action
        marlin-deck-runtime-dynamic-strategy-policy-receipt
        marlin-deck-runtime-strategy-rule-signal-names
        marlin-deck-runtime-dynamic-strategy-rule-signal-names
        marlin-deck-runtime-strategy-selection)
