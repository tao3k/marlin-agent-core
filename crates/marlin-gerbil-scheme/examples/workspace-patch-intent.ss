(import :marlin/protocol)

(def patch
  (make-marlin-workspace-patch
   "gerbil intent"
   "gerbil"
   (list (make-marlin-set-todo-op "memory.org:1:goal" "Done")
         (make-marlin-set-property-op "memory.org:1:goal" "OWNER" "gerbil")
         (make-marlin-mark-memory-candidate-op "memory.org:1:goal" "long-term"))))

(def intent
  (make-marlin-workspace-patch-intent "intent:memory" patch #t))

(def artifact
  (make-marlin-workspace-patch-intent-artifact intent))

(display "workspace-patch-intent-artifact")
(newline)
