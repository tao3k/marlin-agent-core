;;; Deck Module and owner-local runtime contracts.

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/mop Any Fun Object Type.)
        (only-in :clan/poo/number UInt)
        (only-in :clan/poo/type List Record String))

(export DeckScopeContract DeckTurn DeckActorRuntime)

(def DeckMetadata (Record module-id: [String] version: [UInt]))
(def DeckTurn
  (Record policies: [(List Object)] rules: [(List Object)] facts: [Object]
          command: [String] agent-scope: [String]))
(def TurnVector (.cc Type. '.element? vector?))
(def DeckActorRuntime
  (Record evaluate-batch: [(Fun TurnVector <- TurnVector UInt)]
          close: [(Fun Any <-)]))
(def DeckScopeContract
  (.cc (Record metadata: [DeckMetadata] open: [(Fun DeckActorRuntime <- UInt)])
       identity: 'marlin/deck-runtime))
