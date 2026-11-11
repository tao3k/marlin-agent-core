;;; Deck owns execution of typed Context batches on Scheme actors.

(.def DeckRuntime
  identity: 'deck-runtime
  core: (.o requires: (.o Context: (.o module-id: 'context))
            scope-contract: DeckScopeContract
            exports: DeckScope)
  features: (.o))
