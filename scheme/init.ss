;;; Select Lab's Context and Marlin's policy in one Yuanxu declaration.

(yuanxu!
  :load-modules "modules"
  package: "github.com/tao3k/agentic-ai-lab"
  mode: explicit
  rename: Lab
  (ai-agentic-context)
  :load-modules "."
  mode: explicit
  rename: Marlin
  (policy))
