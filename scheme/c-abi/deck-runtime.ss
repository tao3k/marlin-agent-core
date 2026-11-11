;;; Rooted typed entry points for the single-VM Deck Actor runtime.

;; Root the two selected runtime roles in the linked compiler graph. Yuanxu
;; still owns their checked publication; its role loader reuses these builtins.
(import (only-in :marlin-agent-core/deck-runtime/objects DeckOperations)
        (only-in :marlin-agent-core/context/objects MarlinContextOperations)
        :gerbil-scheme-rust/scheme/runtime
        (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports))

(export main)
(def (main . _) (void))
(def deck #f)
(def context-bridge #f)

(def (open _)
  (when deck (error "Deck runtime is already open"))
  (set! context-bridge (.get marlin-agent-core.init.exports Marlin context bridge))
  (set! deck ((.get marlin-agent-core.init.exports Marlin deck-runtime open)
              (.get marlin-agent-core.init.exports Marlin context admit-scope)
              (.get context-bridge encode-scopes)))
  #t)

(def (submit groups)
  (unless deck (error "Deck runtime is closed"))
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
  ((.get deck submit-batch) rows))

(def (receive maximum)
  (unless deck (error "Deck runtime is closed"))
  ((.get deck collect) maximum))

(def (close _)
  (unless deck (error "Deck runtime is closed"))
  (let (statistics ((.get deck close)))
    (set! deck #f)
    (set! context-bridge #f)
    statistics))

(begin-foreign
  (c-define (deck-open root) (int64) int64 "marlin_deck_runtime_open" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root marlin-agent-core/c-abi/deck-runtime#open))
  (c-define (deck-submit root) (int64) int64 "marlin_deck_runtime_submit" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root marlin-agent-core/c-abi/deck-runtime#submit))
  (c-define (deck-receive root) (int64) int64 "marlin_deck_runtime_receive" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root marlin-agent-core/c-abi/deck-runtime#receive))
  (c-define (deck-close root) (int64) int64 "marlin_deck_runtime_close" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root marlin-agent-core/c-abi/deck-runtime#close)))
