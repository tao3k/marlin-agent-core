;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Host wake bytes enter Gambit's native I/O wait, as in Scheme Rust services.
(import (only-in :clan/poo/object .o))
(export open-deck-ingress)
(def (open-deck-ingress read-fd write-fd notify)
  (def input (open-input-file (string-append "/dev/fd/" (number->string read-fd))))
  (def output
    (with-catch (lambda (failure) (close-input-port input) (raise failure))
      (lambda () (open-output-file (string-append "/dev/fd/" (number->string write-fd))))))
  (def closed? #f)
  (def reader
    (spawn (lambda ()
      (let loop ()
        (let (byte (read-u8 input))
          (unless (or closed? (eof-object? byte))
            (let drain ((left 4096))
              (when (and (positive? left) (u8-ready? input))
                (read-u8 input) (drain (1- left))))
            (notify 0 0 0 0 0)
            (loop)))))))
  (.o close: (lambda ()
    (unless closed?
      (set! closed? #t)
      (write-u8 0 output) (force-output output)
      (thread-join! reader)
      (close-input-port input) (close-output-port output))
    #t)))
