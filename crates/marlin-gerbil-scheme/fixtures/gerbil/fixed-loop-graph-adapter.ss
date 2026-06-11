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
;; It intentionally supports only a small tokenized source language. A later
;; compiler pass should replace this with a real Gerbil reader boundary.
;;
;; Gerbil-side modules mirror the first Marlin command protocol DTOs and keep
;; the source parser separate from request/response transport.

(load (path-expand "marlin/request.ss" (path-directory (this-source-file))))
(load (path-expand "marlin/parser.ss" (path-directory (this-source-file))))

(let* ((request (read-gerbil-compile-request))
       (_expected (gerbil-compile-request-expected-kind request))
       (source-text (gerbil-compile-request-source-text request))
       (graph (compile-loop-graph source-text)))
  (display-gerbil-compile-response graph)
  (newline))
