;;; Native adapters are a selected Module; Rust owns their ABI readiness.
(.def CAbi
  identity: 'c-abi
  core: (.o requires: (.o Context: (.o module-id: 'context)
                         Ui: (.o module-id: 'ui)
                         DeckRuntime: (.o module-id: 'deck-runtime))
            scope-contract: CAbiScopeContract
            exports: (make-c-abi-scope Ui Context DeckRuntime)
            projections: (list Build))
  features: (.o))
