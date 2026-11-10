;;; Deck publishes runtime metadata and opens actors on demand.

(import (only-in :clan/poo/object .o)
        (only-in "./runtime.ss" open-deck-runtime))

(export DeckOperations)

(def DeckOperations
  (.o metadata: (.o module-id: "deck-runtime" version: 1)
      open: open-deck-runtime))
