;;; Deck Module contracts: one owner, resident Context actors.

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/mop Any Fun Type.)
        (only-in :clan/poo/number UInt)
        (only-in :clan/poo/type Record String)
        (only-in "../context/types.ss" MarlinContextAdmission))

(export DeckScopeContract DeckActorRuntime)

(def DeckMetadata (Record module-id: [String] version: [UInt]))
(def TurnVector (.cc Type. '.element? vector?))
(def DeckActorRuntime
  (Record evaluate-batch: [(Fun TurnVector <- TurnVector UInt)]
          close: [(Fun Any <-)]))
(def DeckScopeContract
  (.cc (Record metadata: [DeckMetadata]
               open: [(Fun DeckActorRuntime <- UInt MarlinContextAdmission)])
       identity: 'marlin/deck-runtime))
