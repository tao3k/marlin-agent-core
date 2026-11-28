;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Deck Module contracts: one owner, on-demand Context actors.

(import (only-in :marlin-agent-core/interface LabContext)
        (only-in :clan/poo/object .get .cc)

        (only-in :clan/poo/type Fun Type.)
        (only-in :clan/poo/number UInt)
        (only-in :clan/poo/type Exactly Record String True))

(export DeckScopeContract DeckActorRuntime)

(def DeckMetadata (Record module-id: [String] version: [UInt]))
(def TurnVector (.cc Type. '.element? vector?))
(def MessageIds (.cc Type. '.element? u64vector?))
(def ScopeBytes (.cc Type. '.element? u8vector?))
(def ScopeEncoder (Fun ScopeBytes <- TurnVector))
(def FactoryProcedure (.cc Type. '.element? procedure?))
(def DeckActorRuntime
  (Record factory: [FactoryProcedure] mailbox-factory: [FactoryProcedure]
          activate: [(Fun True <- UInt)]
          retire: [(Fun TurnVector <- UInt)]
          submit-module-batch: [(Fun MessageIds <- UInt TurnVector)]
          poll: [(Fun TurnVector <- UInt)]
          arm: [(Fun True <- UInt UInt UInt)]
          attach-ingress: [(Fun True <- UInt UInt)]
          collect: [(Fun TurnVector <- UInt)]
          statistics: [(Fun MessageIds <-)]
          close: [(Fun MessageIds <-)]))
(def DeckScopeContract
  (.cc (Record metadata: [DeckMetadata]
               program-entry: [(Exactly 'marlin-agent-core/deck-runtime/entry)]
               program-runtime-imports: [(Exactly '())]
               open: [(Fun DeckActorRuntime <- (.get LabContext admission-type) ScopeEncoder)])
       identity: 'marlin/deck-runtime))
