;;; Select the modules owned by Marlin.

(yuanxu!
  :load-modules "modules"
  package: "github.com/tao3k/agentic-ai-lab"
  mode: explicit
  rename: Lab
  (ai-agentic-context)
  :load-modules "."
  mode: explicit
  rename: Marlin
  (context +causal)
  (hooks)
  (policy)
  (deck-runtime))
