;;; Validate the Context publication.

(import (only-in :clan/poo/mop validate)
        "./types.ss"
        "./objects.ss")

(export MarlinContextScope)

(def MarlinContextScope
  (validate MarlinContextModuleContract MarlinContextOperations))
