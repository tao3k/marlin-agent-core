;;; Publish the configured Hook selector.

(import (only-in :clan/poo/mop validate)
        "./types.ss"
        "./objects.ss")

(export HooksScope)

(def HooksScope
  (validate HooksScopeContract HooksOperations))
