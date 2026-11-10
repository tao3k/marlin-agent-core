;;; Deck owns execution of admitted policy batches on Scheme actors.

(.def DeckRuntime
  identity: 'deck-runtime
  core: (.o requires: (.o Policy: (.o module-id: 'policy))
            scope-contract: DeckScopeContract
            exports: DeckScope)
  features: (.o))
