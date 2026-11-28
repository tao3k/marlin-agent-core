;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Modular Markdown projector for `scheme/ui-bridge`, emitting the exact same
;;; uniform `doc.block` Document IR as `org-module.ss` so Go renders both Org
;;; Mode and Markdown on one 120Hz GPU pipeline with zero Go-side parsers.
(import (only-in :clan/poo/object .o)
        "./record.ss")
(export MarkdownBridgeModule
        markdown-project-blocks)

(def (md-split-lines text)
  (let ((len (string-length text)))
    (let loop ((i 0) (start 0) (acc '()))
      (cond
        ((>= i len)
         (reverse (cons (substring text start len) acc)))
        ((char=? (string-ref text i) #\newline)
         (let ((line-end (if (and (> i start) (char=? (string-ref text (- i 1)) #\return))
                           (- i 1)
                           i)))
           (loop (+ i 1) (+ i 1) (cons (substring text start line-end) acc))))
        (else
         (loop (+ i 1) start acc))))))

(def (heading-level line)
  (let ((len (string-length line)))
    (let loop ((i 0))
      (cond
        ((and (< i len) (< i 6) (char=? (string-ref line i) #\#))
         (loop (+ i 1)))
        ((and (> i 0) (< i len) (char=? (string-ref line i) #\space))
         i)
        (else 0)))))

(def (horizontal-rule? trimmed)
  (and (>= (string-length trimmed) 3)
       (let ((ch (string-ref trimmed 0)))
         (and (or (char=? ch #\-) (char=? ch #\*) (char=? ch #\_))
              (let loop ((i 0))
                (cond
                  ((>= i (string-length trimmed)) #t)
                  ((or (char=? (string-ref trimmed i) ch) (char=? (string-ref trimmed i) #\space))
                   (loop (+ i 1)))
                  (else #f)))))))

(def (table-row? trimmed)
  (and (>= (string-length trimmed) 2)
       (char=? (string-ref trimmed 0) #\|)
       (char=? (string-ref trimmed (- (string-length trimmed) 1)) #\|)))

(def (table-separator-row? trimmed)
  (and (table-row? trimmed)
       (let loop ((i 0))
         (cond
           ((>= i (string-length trimmed)) #t)
           ((let ((c (string-ref trimmed i)))
              (or (char=? c #\|) (char=? c #\-) (char=? c #\:) (char=? c #\space)))
            (loop (+ i 1)))
           (else #f)))))

(def (split-pipe-cells trimmed)
  (let* ((inner (substring trimmed 1 (- (string-length trimmed) 1)))
         (len (string-length inner)))
    (let loop ((i 0) (start 0) (acc '()))
      (cond
        ((>= i len)
         (reverse (cons (str-trim (substring inner start len)) acc)))
        ((char=? (string-ref inner i) #\|)
         (loop (+ i 1) (+ i 1) (cons (str-trim (substring inner start i)) acc)))
        (else
         (loop (+ i 1) start acc))))))

;; Parse `- [x] text`, `- [ ] text`, `- text`, `* text`, `1. text`.
(def (parse-list-item trimmed)
  (let ((len (string-length trimmed)))
    (cond
      ((and (>= len 6)
            (or (string-prefix? "- [x] " trimmed) (string-prefix? "- [X] " trimmed)
                (string-prefix? "* [x] " trimmed) (string-prefix? "* [X] " trimmed)))
       (list "-" "X" (str-trim (substring trimmed 6 len))))
      ((and (>= len 6)
            (or (string-prefix? "- [ ] " trimmed) (string-prefix? "* [ ] " trimmed)))
       (list "-" " " (str-trim (substring trimmed 6 len))))
      ((and (>= len 2)
            (or (string-prefix? "- " trimmed) (string-prefix? "* " trimmed) (string-prefix? "+ " trimmed)))
       (list "-" "" (str-trim (substring trimmed 2 len))))
      (else
       ;; Check numbered list `1. item`
       (let loop ((i 0))
         (cond
           ((and (< i len) (char-numeric? (string-ref trimmed i)))
            (loop (+ i 1)))
           ((and (> i 0) (< (+ i 1) len)
                 (char=? (string-ref trimmed i) #\.)
                 (char=? (string-ref trimmed (+ i 1)) #\space))
            (list (substring trimmed 0 (+ i 1))
                  ""
                  (str-trim (substring trimmed (+ i 2) len))))
           (else #f)))))))

(def (extract-alert-kind line)
  (let ((t (str-trim line)))
    (cond
      ((string-prefix? "[!NOTE]" t) (cons "NOTE" (str-trim (substring t 7 (string-length t)))))
      ((string-prefix? "[!TIP]" t) (cons "TIP" (str-trim (substring t 6 (string-length t)))))
      ((string-prefix? "[!IMPORTANT]" t) (cons "IMPORTANT" (str-trim (substring t 12 (string-length t)))))
      ((string-prefix? "[!WARNING]" t) (cons "WARNING" (str-trim (substring t 10 (string-length t)))))
      ((string-prefix? "[!CAUTION]" t) (cons "CAUTION" (str-trim (substring t 10 (string-length t)))))
      (else (cons "" t)))))

(def (markdown-project-blocks path source)
  (let ((lines (list->vector (md-split-lines source)))
        (out '())
        (block-idx 0))
    (def n (vector-length lines))
    (def (emit! kind-str title summary start-ln end-ln extra-facets)
      (let* ((id (string-append "doc:block:" (number->string block-idx)))
             (facets (cons (cons "block_kind" kind-str)
                           (cons (cons "format" "markdown") extra-facets))))
        (set! out (cons (make-bridge-record-vec id "doc.block" title summary path start-ln end-ln facets) out))
        (set! block-idx (+ block-idx 1))))

    (let loop ((i 0))
      (when (< i n)
        (let* ((line (vector-ref lines i))
               (trimmed (str-trim line)))
          (cond
            ((string=? trimmed "")
             (loop (+ i 1)))

            ;; Fenced code block ```lang
            ((string-prefix? "```" trimmed)
             (let ((lang (let ((rest (str-trim (substring trimmed 3 (string-length trimmed)))))
                           (if (string=? rest "") "text" rest))))
               (let code-loop ((j (+ i 1)) (acc '()))
                 (cond
                   ((>= j n)
                    (emit! "code" lang (str-join (reverse acc) "\n") (+ i 1) n (list (cons "lang" lang)))
                    (loop n))
                   ((string-prefix? "```" (str-trim (vector-ref lines j)))
                    (emit! "code" lang (str-join (reverse acc) "\n") (+ i 1) (+ j 1) (list (cons "lang" lang)))
                    (loop (+ j 1)))
                   (else
                    (code-loop (+ j 1) (cons (vector-ref lines j) acc)))))))

            ;; Heading #..######
            ((> (heading-level trimmed) 0)
             (let* ((lvl (heading-level trimmed))
                    (title (str-trim (substring trimmed (+ lvl 1) (string-length trimmed)))))
               (emit! "heading" title title (+ i 1) (+ i 1)
                      (list (cons "level" (number->string lvl))
                            (cons "is_org" "false")))
               (loop (+ i 1))))

            ;; Horizontal rule
            ((horizontal-rule? trimmed)
             (emit! "rule" "---" "" (+ i 1) (+ i 1) '())
             (loop (+ i 1)))

            ;; GFM Table
            ((and (table-row? trimmed)
                  (< (+ i 1) n)
                  (table-separator-row? (str-trim (vector-ref lines (+ i 1)))))
             (let ((hdr (str-join (split-pipe-cells trimmed) "\t")))
               (let tbl-loop ((j (+ i 2)) (rows '()))
                 (if (and (< j n) (table-row? (str-trim (vector-ref lines j))))
                   (tbl-loop (+ j 1)
                             (cons (str-join (split-pipe-cells (str-trim (vector-ref lines j))) "\t") rows))
                   (begin
                     (emit! "table" "Table" "" (+ i 1) j
                            (list (cons "headers" hdr)
                                  (cons "rows" (str-join (reverse rows) "\n"))))
                     (loop j))))))

            ;; Blockquote / GitHub Alert
            ((string-prefix? ">" trimmed)
             (let q-loop ((j i) (q-lines '()))
               (if (and (< j n) (string-prefix? ">" (str-trim (vector-ref lines j))))
                 (let* ((t (str-trim (vector-ref lines j)))
                        (body (str-trim (substring t 1 (string-length t)))))
                   (q-loop (+ j 1) (cons body q-lines)))
                 (let* ((ordered (reverse q-lines))
                        (first-parsed (if (pair? ordered) (extract-alert-kind (car ordered)) (cons "" "")))
                        (alert (car first-parsed))
                        (rest-lines (if (string=? alert "")
                                      ordered
                                      (if (string=? (cdr first-parsed) "")
                                        (cdr ordered)
                                        (cons (cdr first-parsed) (cdr ordered))))))
                   (emit! "quote" (if (string=? alert "") "Quote" alert)
                          (str-join rest-lines "\n")
                          (+ i 1) j
                          (if (string=? alert "") '() (list (cons "alert_kind" alert))))
                   (loop j)))))

            ;; List / checklist
            ((parse-list-item trimmed)
             (let l-loop ((j i) (items '()))
               (if (< j n)
                 (let ((parsed (parse-list-item (str-trim (vector-ref lines j)))))
                   (if parsed
                     (l-loop (+ j 1)
                             (cons (string-append (car parsed) "|" (cadr parsed) "|" (caddr parsed)) items))
                     (begin
                       (emit! "list" "List" "" (+ i 1) j
                              (list (cons "items" (str-join (reverse items) "\n"))))
                       (loop j))))
                 (begin
                   (emit! "list" "List" "" (+ i 1) j
                          (list (cons "items" (str-join (reverse items) "\n"))))
                   (loop j)))))

            ;; Paragraph (merge contiguous non-blank plain lines)
            (else
             (let p-loop ((j (+ i 1)) (p-lines (list trimmed)))
               (if (< j n)
                 (let ((nt (str-trim (vector-ref lines j))))
                   (if (or (string=? nt "")
                           (string-prefix? "```" nt)
                           (string-prefix? ">" nt)
                           (> (heading-level nt) 0)
                           (horizontal-rule? nt)
                           (table-row? nt)
                           (parse-list-item nt))
                     (begin
                       (emit! "paragraph" "" (str-join (reverse p-lines) " ") (+ i 1) j '())
                       (loop j))
                     (p-loop (+ j 1) (cons nt p-lines))))
                 (begin
                   (emit! "paragraph" "" (str-join (reverse p-lines) " ") (+ i 1) j '())
                   (loop j)))))))))
    (reverse out)))

(def (markdown-handle-query _verb arg payload)
  (markdown-project-blocks arg payload))

(def MarkdownBridgeModule
  (.o namespace: "markdown"
      owner-id: "marlin.scheme.markdown"
      description: "Modular Gerbil Scheme Markdown projector emitting unified doc.block IR"
      handle-query: markdown-handle-query))
