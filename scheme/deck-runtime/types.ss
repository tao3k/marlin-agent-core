;;; Deck Module contracts: one owner, on-demand Context actors.

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/mop Fun Type.)
        (only-in :clan/poo/number UInt)
        (only-in :clan/poo/type Record String)
        (only-in "../context/types.ss" MarlinContextAdmission))

(export DeckScopeContract DeckActorRuntime)

(def DeckMetadata (Record module-id: [String] version: [UInt]))
(def TurnVector (.cc Type. '.element? vector?))
(def MessageIds (.cc Type. '.element? u64vector?))
(def ScopeBytes (.cc Type. '.element? u8vector?))
(def ScopeEncoder (Fun ScopeBytes <- TurnVector))
(def DeckActorRuntime
  (Record submit-batch: [(Fun MessageIds <- TurnVector)]
          collect: [(Fun TurnVector <- UInt)]
          statistics: [(Fun MessageIds <-)]
          close: [(Fun MessageIds <-)]))
(def DeckScopeContract
  (.cc (Record metadata: [DeckMetadata]
               open: [(Fun DeckActorRuntime <- MarlinContextAdmission ScopeEncoder)])
       identity: 'marlin/deck-runtime))
