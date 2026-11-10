;;; Assemble the operations of the Hooks module.

(import (only-in :clan/poo/object .o)
        (only-in "./selection.ss" select-hook))

(export HooksOperations)

(def HooksOperations
  (.o select: select-hook))
