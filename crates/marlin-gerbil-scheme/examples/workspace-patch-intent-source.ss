(workspace-patch-intent "intent:memory"
  (dry-run-first #t)
  (patch
    (reason "gerbil intent")
    (source-agent "gerbil")
    (set-todo "memory.org:1:goal" DONE)
    (set-property "memory.org:1:goal" OWNER "gerbil")
    (mark-memory-candidate "memory.org:1:goal" "long-term")))
