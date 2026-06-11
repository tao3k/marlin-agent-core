;; Minimal Gerbil command-adapter shim.
;;
;; The Rust side writes a JSON GerbilCompileRequest to stdin. This executable
;; shim decodes the request, extracts the source text, and recognizes the first
;; smoke form:
;;
;;   (loop-graph gerbil-source-loop
;;     (node provider ask-model (config role planner))
;;     (node tool run-tool (config mode execute))
;;     (edge provider tool success)
;;     (edge tool provider none))
;;
;; The source parser is Gerbil reader-backed and intentionally supports only
;; the first loop graph DSL subset.
;;
;; Run this launcher with GERBIL_LOADPATH pointing at this fixture directory so
;; `:marlin/adapter` resolves as a Gerbil library module.

(import (only-in :marlin/adapter run-fixed-loop-graph-adapter))

(run-fixed-loop-graph-adapter)
