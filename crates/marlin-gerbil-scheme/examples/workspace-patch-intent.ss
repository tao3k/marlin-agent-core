;;; -*- Gerbil -*-
;;; Run with:
;;;   GERBIL_LOADPATH=<runtime-asset-root> gxi workspace-patch-intent.ss

(import :marlin/parser :marlin/protocol)

(def workspace-patch-intent-source
  "(workspace-patch-intent \"intent:memory\"
     (dry-run-first #t)
     (patch
       (reason \"gerbil intent\")
       (source-agent \"gerbil\")
       (set-todo \"memory.org:1:goal\" DONE)
       (set-property \"memory.org:1:goal\" OWNER \"gerbil\")
       (mark-memory-candidate \"memory.org:1:goal\" \"long-term\")))")

(def artifact
  (compile-workspace-patch-intent workspace-patch-intent-source))

(display-gerbil-artifact-response
 marlin-workspace-patch-intent-artifact-kind
 artifact)
(newline)
