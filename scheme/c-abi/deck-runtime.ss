;;; Rooted typed entry points for the single-VM Deck Actor runtime.

(import :gerbil-scheme-rust/scheme/runtime
        (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports))

(export main)
(def (main . _) (void))
(def deck #f)
(def context-bridge #f)

(def (open worker-count)
  (when deck (error "Deck runtime is already open"))
  (set! context-bridge (.get marlin-agent-core.init.exports Marlin context bridge))
  (set! deck ((.get marlin-agent-core.init.exports Marlin deck-runtime open)
              worker-count (.get marlin-agent-core.init.exports Marlin context admit-scope)))
  #t)

(def (evaluate input)
  (unless deck (error "Deck runtime is closed"))
  (unless (= (vector-length input) 2) (error "invalid Deck batch envelope"))
  (def groups (vector-ref input 1))
  (unless (<= 1 (vector-length groups) 64) (error "invalid Deck group count"))
  (def decoded (vector-map (.get context-bridge decode-batch) groups))
  (def count (foldl (lambda (rows total) (+ total (vector-length rows))) 0 (vector->list decoded)))
  (unless (<= 1 count 64) (error "invalid Deck row count"))
  (def rows (make-vector count))
  (let loop ((group 0) (offset 0))
    (when (< group (vector-length decoded))
      (let (part (vector-ref decoded group))
        (vector-copy! rows offset part)
        (loop (1+ group) (+ offset (vector-length part))))))
  ((.get context-bridge encode-scopes) ((.get deck evaluate-batch) rows (vector-ref input 0))))

(def (close _)
  (when deck ((.get deck close)) (set! deck #f) (set! context-bridge #f))
  #t)

(begin-foreign
  (c-define (deck-open root) (int64) int64 "marlin_deck_runtime_open" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root marlin-agent-core/c-abi/deck-runtime#open))
  (c-define (deck-call root) (int64) int64 "marlin_deck_runtime_evaluate" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root marlin-agent-core/c-abi/deck-runtime#evaluate))
  (c-define (deck-close root) (int64) int64 "marlin_deck_runtime_close" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root marlin-agent-core/c-abi/deck-runtime#close)))
