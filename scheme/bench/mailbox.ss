;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Production mailbox transport, timed inside Gerbil after startup and warmup.
package: marlin-benchmark

(import (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/deck-runtime/mailbox
                 open-deck-mailboxes deck-reply-id deck-reply-failed? deck-reply-value))

(export run-mailbox-benchmark mailbox-clock-ns)

(extern mailbox-clock-ns)
(begin-foreign
  (namespace ("marlin-benchmark/mailbox#" mailbox-clock-ns))
  (c-declare #<<c-end
#include <stdint.h>
#include <time.h>
static uint64_t marlin_mailbox_clock(void) {
  struct timespec now;
#ifdef __APPLE__
  if (clock_gettime(CLOCK_UPTIME_RAW, &now)) return UINT64_MAX;
#else
  if (clock_gettime(CLOCK_MONOTONIC, &now)) return UINT64_MAX;
#endif
  return (uint64_t)now.tv_sec * 1000000000ULL + (uint64_t)now.tv_nsec;
}
c-end
  )
  (define mailbox-clock-ns (c-lambda () unsigned-int64 "marlin_mailbox_clock")))

(def (run-mailbox-benchmark request)
  (unless (= (u64vector-length request) 3) (error "invalid mailbox benchmark request"))
  (def count (u64vector-ref request 0))
  (def identities (u64vector-ref request 1))
  (def arrival-rate (u64vector-ref request 2))
  (def latency? (positive? arrival-rate))
  (def window 128)
  (unless (and (integer? count) (positive? count)
               (integer? identities) (positive? identities)
               (integer? arrival-rate) (>= arrival-rate 0))
    (error "message and identity counts must be positive integers"))
  (def mailboxes (open-deck-mailboxes (lambda (_) 0) car cdr))
  ((.get mailboxes activate) 0)
  (def dispatch (.get mailboxes submit))
  (def collect (.get mailboxes collect))
  (def timestamps (and latency? (make-u64vector count)))
  (def seen (make-u8vector (+ count 1000) 0))
  ;; Ten-microsecond buckets through one second, plus an explicit overflow bin.
  (def histogram (and latency? (make-u64vector 100001 0)))
  (def maximum-ns 0)
  (def over-budget 0)
  (def (clock)
    (def now (mailbox-clock-ns))
    (when (= now 18446744073709551615) (error "monotonic clock unavailable"))
    now)
  (def (record-reply reply start)
    (def index (- (1- (deck-reply-id reply)) start))
    (def elapsed (- (clock) (u64vector-ref timestamps index)))
    (def bucket (min 100000 (quotient elapsed 10000)))
    (u64vector-set! histogram bucket (1+ (u64vector-ref histogram bucket)))
    (set! maximum-ns (max maximum-ns elapsed))
    (when (> elapsed 20000000) (set! over-budget (1+ over-budget))))
  (def (run start length record?)
    (def origin (and record? (clock)))
    (def (planned index) (+ origin (quotient (* index 1000000000) arrival-rate)))
    (let loop ((sent 0) (received 0))
      (if (= received length) (void)
        (let fill ((sent sent))
          (if (and (< sent length) (< (- sent received) window)
                   (or (not record?) (>= (clock) (planned sent))))
            (let (ordinal (+ start sent))
              (when record? (u64vector-set! timestamps sent (planned sent)))
              (let (id (dispatch (cons (modulo ordinal identities) ordinal)))
                (unless (= id (1+ ordinal)) (error "mailbox submission identity changed")))
              (fill (1+ sent)))
            (let (ready (collect window))
              (vector-for-each
                (lambda (reply)
                  (when (deck-reply-failed? reply) (error "mailbox handler failed"))
                  (let (ordinal (1- (deck-reply-id reply)))
                    (unless (and (<= start ordinal) (< ordinal (+ start length))
                                 (= (deck-reply-value reply) ordinal)
                                 (zero? (u8vector-ref seen ordinal)))
                      (error "mailbox reply identity or oracle mismatch"))
                    (u8vector-set! seen ordinal 1))
                  (when record? (record-reply reply start)))
                ready)
              (loop sent (+ received (vector-length ready)))))))))
  (run 0 1000 #f)
  (def before (##process-statistics))
  (run 1000 count latency?)
  (def after (##process-statistics))
  (def wall (- (f64vector-ref after 2) (f64vector-ref before 2)))
  (def statistics ((.get mailboxes close)))
  (unless (and (= (u64vector-ref statistics 0) (min identities (+ count 1000)))
               (= (u64vector-ref statistics 0) (u64vector-ref statistics 1))
               (= (u64vector-ref statistics 2) (+ count 1000))
               (= (u64vector-ref statistics 3) (+ count 1000)))
    (error "mailbox shutdown accounting failed"))
  (def (quantile numerator denominator)
    (def rank (quotient (+ (* count numerator) (1- denominator)) denominator))
    (let loop ((bucket 0) (seen 0))
      (let (seen (+ seen (u64vector-ref histogram bucket)))
        (if (>= seen rank)
          (if (= bucket 100000) maximum-ns (* (1+ bucket) 10000))
          (loop (1+ bucket) seen)))))
  (def (ns value) (inexact->exact (round (* value 1000000000.0))))
  ;; Field 7 is stack-sensitive VM accounting, not physical constructor bytes.
  ;; Latency and accounting fields remain independently checked by Rust.
  (s64vector count identities (ns wall)
    (ns (+ (- (f64vector-ref after 0) (f64vector-ref before 0))
           (- (f64vector-ref after 1) (f64vector-ref before 1))))
    (ns (+ (- (f64vector-ref after 3) (f64vector-ref before 3))
           (- (f64vector-ref after 4) (f64vector-ref before 4))))
    (inexact->exact (- (f64vector-ref after 7) (f64vector-ref before 7)))
    (u64vector-ref statistics 0) (u64vector-ref statistics 1)
    (if latency? (quantile 50 100) 0)
    (if latency? (quantile 95 100) 0)
    (if latency? (quantile 99 100) 0)
    (if latency? (quantile 999 1000) 0)
    maximum-ns over-budget
    (inexact->exact (- (f64vector-ref after 6) (f64vector-ref before 6)))
    (inexact->exact (f64vector-ref after 15))
    (inexact->exact (f64vector-ref after 17))))
