;;; Select the modules owned by Marlin.

(yuanxu!
  :load-modules "."
  mode: explicit
  rename: Marlin
  (context +causal)
  (hooks)
  (policy)
  (deck-runtime))
