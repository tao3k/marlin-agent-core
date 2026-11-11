(import (only-in :clan/poo/mop validate) "./types.ss" "./objects.ss")
(export UiScope)
(def UiScope (validate UiScopeContract UiOperations))
