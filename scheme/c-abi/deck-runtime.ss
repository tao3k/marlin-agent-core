;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Rooted typed entry points for the single-VM Deck Actor runtime.

;; Root the two selected runtime roles in the linked compiler graph. Yuanxu
;; still owns their checked publication; its role loader reuses these builtins.
(import (only-in :marlin-agent-core/deck-runtime/objects DeckOperations)
        (only-in :marlin-agent-core/context/objects MarlinContextOperations)
        (only-in :marlin-agent-core/deck-runtime/runtime check-deck-kernel!)
        :gerbil-scheme-rust/scheme/runtime
        (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports))

(export main)
(def (main . _) (void))
(def deck #f)
(def context-bridge #f)

(def (open wake-fds)
  (when deck (error "Deck runtime is already open"))
  (def runtime (.get marlin-agent-core.init.exports Marlin deck-runtime))
  (def metadata (.get runtime metadata))
  (def module-id (.get metadata module-id))
  (when (zero? (string-length module-id)) (error "Deck runtime module ID is empty"))
  (def receipt (vector (string-copy module-id)
                       (u64vector (.get metadata version))))
  (set! context-bridge (.get marlin-agent-core.init.exports Marlin context bridge))
  (set! deck ((.get runtime open)
              (.get marlin-agent-core.init.exports Marlin context admit-scope)
              (.get context-bridge encode-scopes)))
  ;; The actual constructed runtime must use the linked library procedures.
  ;; Compiler manifests alone cannot certify a reader's private interpreted copy.
  (check-deck-kernel! deck)
  ((.get deck attach-ingress) (u64vector-ref wake-fds 0) (u64vector-ref wake-fds 1))
  receipt)

(def (submit envelope)
  (def module-ids (vector-ref envelope 0))
  (def groups (vector-ref envelope 1))
  (unless (= (u64vector-length module-ids) (vector-length groups))
    (error "invalid Module projection count"))
  (unless deck (error "Deck runtime is closed"))
  (unless (<= 1 (vector-length groups) 64) (error "invalid Deck group count"))
  (def decoded (vector-map (.get context-bridge decode-batch) groups))
  (def count (foldl (lambda (rows total) (+ total (vector-length rows))) 0 (vector->list decoded)))
  (unless (<= 1 count 64) (error "invalid Deck row count"))
  (def ids (make-u64vector count))
  (let loop ((group 0) (offset 0))
    (when (< group (vector-length decoded))
      (let* ((part (vector-ref decoded group))
             (result ((.get deck submit-module-batch) (u64vector-ref module-ids group) part)))
      (let copy ((i 0))
        (when (< i (u64vector-length result))
          (u64vector-set! ids (+ offset i) (u64vector-ref result i)) (copy (1+ i))))
      (loop (1+ group) (+ offset (vector-length part))))))
  ids)

(def (receive maximum)
  (unless deck (error "Deck runtime is closed"))
  ((.get deck collect) maximum))

(def (activate id) ((.get deck activate) (u64vector-ref id 0)))
(def (retire id) ((.get deck retire) (u64vector-ref id 0)))
(def (arm timer)
  ((.get deck arm) (u64vector-ref timer 0) (u64vector-ref timer 1) (u64vector-ref timer 2)))
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

;; Diagnostic snapshot uses the existing runtime counters, not a wire encoding.
(begin-foreign
  (c-define (deck-statistics root) (int64) int64 "marlin_deck_runtime_statistics" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
      (lambda (_) (##process-statistics)))))

(begin-foreign
  (c-define (deck-activate root) (int64) int64 "marlin_deck_runtime_activate" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root marlin-agent-core/c-abi/deck-runtime#activate)))

(begin-foreign
  (c-define (deck-retire root) (int64) int64 "marlin_deck_runtime_retire" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root marlin-agent-core/c-abi/deck-runtime#retire)))

(begin-foreign
  (c-define (deck-arm root) (int64) int64 "marlin_deck_runtime_arm" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root marlin-agent-core/c-abi/deck-runtime#arm)))
