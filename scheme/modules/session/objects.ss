;;; Default session policy values.

(import (only-in :clan/poo/object .o))

(export SessionDefaults)

(def SessionDefaults
  (.o sharing: "shared" isolation: "branch-isolated" snapshot: "enabled"))
