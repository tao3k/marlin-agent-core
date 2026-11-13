;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Shared typed Context projections for direct and Actor entry points.

(import (only-in :clan/poo/object .get .o)
        (only-in :tao3k/agentic-ai-lab/modules/ai-agentic-context/objects AiAgenticContextScope.)
        (only-in :std/encoding/hex hex-decode-byte))

(export marlin-context-batch-requests marlin-context-scopes->bytes)

(def (marlin-context-batch-requests input)
  (unless (= (vector-length input) 3) (error "invalid Context batch envelope"))
  (def shared (vector-ref input 0))
  (def numbers (vector-ref input 1))
  (def rows (vector-ref input 2))
  (def count (vector-length rows))
  (unless (and (= (vector-length shared) 5) (<= 1 count 64)
               (= (u64vector-length numbers) (1+ count)))
    (error "invalid Context batch size"))
  ;; These are boundary values, not computations over a later self. Reuse the
  ;; standard POO constant slots and one shared prototype for this typed group.
  (def bundle (vector-ref shared 0))
  (def organization (vector-ref shared 1))
  (def epoch (u64vector-ref numbers 0))
  (def project (vector-ref shared 2))
  (def worktree (vector-ref shared 3))
  (def source-cut (vector-ref shared 4))
  (def common (.o (:: @ AiAgenticContextScope.)
                 (bundle) (organization) (epoch) (project) (worktree) (source-cut)))
  ;; Fill the native vector directly; no intermediate row list is needed.
  (def output (make-vector count))
  (let loop ((i 0))
    (when (< i count)
      (let* ((row (vector-ref rows i))
             (session
               (case (vector-length row)
                 ((3) #f)
                 ((4) (let (value (vector-ref row 3))
                        (unless (string? value) (error "invalid Context session ID")) value))
                 (else (error "invalid Context batch row"))))
             (actor (vector-ref row 0))
             (task (vector-ref row 1))
             (destination (vector-ref row 2))
             (turn (and session (u64vector-ref numbers (1+ i)))))
        (vector-set! output i
          (.o (:: @ common) (actor) (task) (destination) (session) (turn)))
        (loop (1+ i)))))
  output)

(def (marlin-context-scopes->bytes scopes)
  (def count (vector-length scopes))
  (unless (<= 1 count 64) (error "invalid Context result count"))
  ;; Decode directly into the fixed-size native buffer.
  (def output (make-u8vector (* count 32)))
  (let loop ((i 0))
    (when (< i count)
      (let (digest (.get (vector-ref scopes i) semantic-digest))
        (unless (and (= (string-length digest) 71)
                     (string=? (substring digest 0 7) "sha256:"))
          (error "unexpected Lab Context digest"))
        (let copy ((j 0))
          (when (< j 32)
            (u8vector-set! output (+ (* i 32) j) (hex-decode-byte digest (+ 7 (* 2 j))))
            (copy (1+ j)))))
      (loop (1+ i))))
  output)
