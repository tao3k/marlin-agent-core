;;; Validate the Deck publication before exposing its operations.

(import (only-in :clan/poo/mop validate)
        "./objects.ss"
        "./types.ss")

(export DeckScope)

(def DeckScope (validate DeckScopeContract DeckOperations))
