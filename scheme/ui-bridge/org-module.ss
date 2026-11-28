;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Modular Org Mode projector for `scheme/ui-bridge`, powered directly by
;;; Gerbil Scheme's authoritative `orgize` / `orgize-fold` parser via
;;; `marlin-context-org-source-graph`. Zero parser duplication in Rust or Go.
(import (only-in :clan/poo/object .get .o)
        (only-in :std/list/list filter filter-map find)
        (only-in :marlin-agent-core/context-system/org-source marlin-context-org-source-graph)
        "./record.ss")
(export OrgBridgeModule
        org-project-blocks
        org-project-headings
        org-project-agenda)

;; Exact UTF-8 byte-span slice (orgize records report UTF-8 byte offsets).
(def (slice-utf8-bytes bytes start end)
  (let* ((len (u8vector-length bytes))
         (s (max 0 (min start len)))
         (e (max s (min end len))))
    (bytes->string (subu8vector bytes s e))))

;; Precompute 1-based line number at byte offset `pos`.
(def (byte-offset->line bytes pos)
  (let ((limit (min pos (u8vector-length bytes))))
    (let loop ((i 0) (line 1))
      (if (>= i limit)
        line
        (loop (+ i 1) (if (= (u8vector-ref bytes i) 10) (+ line 1) line))))))

;; Split string by newline into list of lines.
(def (split-lines text)
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

(def (strip-src-block-wrapper raw)
  (let* ((lines (split-lines raw))
         (non-empty (let loop ((xs lines))
                      (if (and (pair? xs) (string=? (str-trim (car xs)) ""))
                        (loop (cdr xs))
                        xs)))
         (after-begin (if (and (pair? non-empty)
                               (let ((lo (str-downcase (str-trim (car non-empty)))))
                                 (string-prefix? "#+begin_" lo)))
                        (cdr non-empty)
                        non-empty))
         (rev (reverse after-begin))
         (without-end (if (and (pair? rev)
                               (let ((lo (str-downcase (str-trim (car rev)))))
                                 (string-prefix? "#+end_" lo)))
                        (reverse (cdr rev))
                        after-begin)))
    (str-join without-end "\n")))

;; Pure merge sort by comparator `less?`.
(def (merge-sorted xs ys less?)
  (cond
    ((null? xs) ys)
    ((null? ys) xs)
    ((less? (car xs) (car ys))
     (cons (car xs) (merge-sorted (cdr xs) ys less?)))
    (else
     (cons (car ys) (merge-sorted xs (cdr ys) less?)))))

(def (sort-list xs less?)
  (if (or (null? xs) (null? (cdr xs)))
    xs
    (let split ((slow xs) (fast xs) (left '()))
      (if (or (null? fast) (null? (cdr fast)))
        (merge-sorted (sort-list (reverse left) less?)
                      (sort-list slow less?)
                      less?)
        (split (cdr slow) (cddr fast) (cons (car slow) left))))))

;; Build parent-id -> sorted list of child records index.
(def (index-children records)
  (let ((by-parent (make-hash-table)))
    (for-each
      (lambda (rec)
        (let ((p (.get rec parent)))
          (when p
            (hash-put! by-parent p (cons rec (or (hash-get by-parent p) '()))))))
      records)
    (hash-for-each
      (lambda (k v)
        (hash-put! by-parent k (sort-list v (lambda (a b) (< (.get a start) (.get b start))))))
      by-parent)
    by-parent))

(def (safe-field field-of rec name)
  (with-catch (lambda (_) #f) (lambda () (field-of rec name))))

(def (as-string-list v)
  (cond
    ((pair? v) (filter string? v))
    ((string? v) (if (string=? (str-trim v) "") '() (list v)))
    (else '())))

;; Extract inline spans for a paragraph record from its child records.
(def (encode-inline-spans bytes by-parent rec)
  (let ((kids (or (hash-get by-parent (.get rec id)) '())))
    (if (null? kids)
      ""
      (let* ((p-start (.get rec start))
             (p-end (.get rec end)))
        (let loop ((pos p-start) (rem kids) (acc '()))
          (if (null? rem)
            (let* ((tail (if (< pos p-end) (slice-utf8-bytes bytes pos p-end) ""))
                   (final (if (string=? tail "") acc (cons (string-append "plain|" (sanitize-inline tail)) acc))))
              (str-join (reverse final) "\n"))
            (let* ((k (car rem))
                   (ks (.get k start))
                   (ke (.get k end))
                   (kind (.get k kind))
                   (pre (if (< pos ks) (slice-utf8-bytes bytes pos ks) ""))
                   (acc1 (if (string=? pre "") acc (cons (string-append "plain|" (sanitize-inline pre)) acc)))
                   (raw-k (slice-utf8-bytes bytes ks ke))
                   (span-str
                     (cond
                       ((equal? kind "bold")
                        (string-append "bold|" (sanitize-inline (trim-delims raw-k #\*))))
                       ((equal? kind "italic")
                        (string-append "italic|" (sanitize-inline (trim-delims raw-k #\/))))
                       ((or (equal? kind "code") (equal? kind "verbatim"))
                        (string-append "code|" (sanitize-inline (trim-delims raw-k (if (equal? kind "code") #\~ #\=)))))
                       ((equal? kind "link")
                        (encode-org-link raw-k))
                       (else
                        (string-append "plain|" (sanitize-inline raw-k))))))
              (loop (max pos ke) (cdr rem) (cons span-str acc1)))))))))

(def (sanitize-inline s)
  (let* ((len (string-length s))
         (out (make-string len)))
    (let loop ((i 0))
      (if (>= i len)
        out
        (let ((ch (string-ref s i)))
          (string-set! out i (if (or (char=? ch #\newline) (char=? ch #\return)) #\space ch))
          (loop (+ i 1)))))))

(def (trim-delims s ch)
  (let ((len (string-length s)))
    (if (and (>= len 2) (char=? (string-ref s 0) ch) (char=? (string-ref s (- len 1)) ch))
      (substring s 1 (- len 1))
      s)))

;; Parse `[[url][desc]]` or `[[url]]` into `link|desc|url`.
(def (encode-org-link raw)
  (let* ((trimmed (str-trim raw))
         (len (string-length trimmed)))
    (if (and (>= len 4) (string-prefix? "[[" trimmed))
      (let* ((inner (substring trimmed 2 (- len 2)))
             (sep (string-contains-pair inner "][")))
        (if sep
          (let ((url (substring inner 0 sep))
                (desc (substring inner (+ sep 2) (string-length inner))))
            (string-append "link|" (sanitize-inline desc) "|" (sanitize-inline url)))
          (string-append "link|" (sanitize-inline inner) "|" (sanitize-inline inner))))
      (string-append "plain|" (sanitize-inline trimmed)))))

(def (string-contains-pair s pat)
  (let ((slen (string-length s))
        (plen (string-length pat)))
    (let loop ((i 0))
      (cond
        ((> (+ i plen) slen) #f)
        ((string=? (substring s i (+ i plen)) pat) i)
        (else (loop (+ i 1)))))))

;; Project a single Org `table` record into `(headers-str . rows-str)`.
(def (extract-org-table bytes by-parent table-rec)
  (let* ((row-recs (or (hash-get by-parent (.get table-rec id)) '()))
         (data-rows
           (filter-map
             (lambda (row-rec)
               (if (equal? (.get row-rec kind) "table-row")
                 (let ((cells (filter (lambda (c) (equal? (.get c kind) "table-cell"))
                                      (or (hash-get by-parent (.get row-rec id)) '()))))
                   (map (lambda (c)
                          (sanitize-inline (str-trim (slice-utf8-bytes bytes (.get c start) (.get c end)))))
                        cells))
                 #f))
             row-recs)))
    (if (null? data-rows)
      (cons "" "")
      (let ((hdr (str-join (car data-rows) "\t"))
            (body (str-join (map (lambda (r) (str-join r "\t")) (cdr data-rows)) "\n")))
        (cons hdr body)))))

;; Project a `plain-list` record into newline-delimited `marker|check|text` items.
(def (extract-org-list bytes by-parent field-of list-rec)
  (let* ((items (filter (lambda (r) (equal? (.get r kind) "item"))
                        (or (hash-get by-parent (.get list-rec id)) '()))))
    (str-join
      (map (lambda (item)
             (let* ((chk-raw (safe-field field-of item "checkbox"))
                    (chk (if (string? chk-raw) chk-raw ""))
                    (kids (or (hash-get by-parent (.get item id)) '()))
                    (para (find (lambda (k) (equal? (.get k kind) "paragraph")) kids))
                    (txt (if para
                           (sanitize-inline (str-trim (slice-utf8-bytes bytes (.get para start) (.get para end))))
                           (sanitize-inline (str-trim (slice-utf8-bytes bytes (.get item start) (.get item end)))))))
               (string-append "-|" chk "|" txt)))
           items)
      "\n")))

;; Walk the Org element tree top-down in source order emitting uniform `doc.block` vectors.
(def (org-project-blocks path source)
  (let* ((projected (marlin-context-org-source-graph source))
         (owned (.get projected source))
         (bytes (string->bytes owned))
         (g (.get projected graph))
         (records (.get g records))
         (field-of (.get g field-of))
         (by-parent (index-children records))
         (root (find (lambda (r) (equal? (.get r kind) "org-data")) records))
         (out '())
         (idx 0))
    (def (emit! kind-str title summary rec extra-facets)
      (let* ((line (byte-offset->line bytes (.get rec start)))
             (end-line (byte-offset->line bytes (.get rec end)))
             (id (string-append "doc:block:" (number->string idx)))
             (facets (cons (cons "block_kind" kind-str)
                           (cons (cons "format" "org") extra-facets))))
        (set! out (cons (make-bridge-record-vec id "doc.block" title summary path line end-line facets) out))
        (set! idx (+ idx 1))))

    (def (visit rec)
      (let ((k (.get rec kind)))
        (cond
          ((equal? k "org-data")
           (for-each visit (or (hash-get by-parent (.get rec id)) '())))

          ((equal? k "keyword")
           (let ((kw (safe-field field-of rec "key"))
                 (val (safe-field field-of rec "value")))
             (when (and (string? kw) (string? val) (string=? (string-downcase kw) "title"))
               (emit! "heading" val val rec
                      '(("level" . "1") ("is_org" . "true") ("doc_title" . "true"))))))

          ((equal? k "headline")
           (let* ((lvl-raw (safe-field field-of rec "level"))
                  (lvl (if (string? lvl-raw) lvl-raw "1"))
                  (title (or (safe-field field-of rec "title") ""))
                  (todo (or (safe-field field-of rec "todo-keyword") ""))
                  (prio (or (safe-field field-of rec "priority") ""))
                  (tags (as-string-list (safe-field field-of rec "tags")))
                  (facets (append
                            (list (cons "level" lvl)
                                  (cons "is_org" "true"))
                            (if (string=? todo "") '() (list (cons "todo_word" todo)))
                            (if (string=? prio "") '() (list (cons "priority" prio)))
                            (if (null? tags) '() (list (cons "tags" (str-join tags ",")))))))
             (emit! "heading" title title rec facets)
             (for-each visit (or (hash-get by-parent (.get rec id)) '()))))

          ((equal? k "planning")
           (let* ((ks (as-string-list (safe-field field-of rec "key")))
                  (vs (as-string-list (safe-field field-of rec "value")))
                  (pairs (let loop ((a ks) (b vs) (acc '()))
                           (if (or (null? a) (null? b))
                             (reverse acc)
                             (loop (cdr a) (cdr b)
                                   (cons (string-append (car a) "|" (car b)) acc))))))
             (unless (null? pairs)
               (emit! "org-planning" "Planning" (str-join pairs " ") rec
                      (list (cons "planning" (str-join pairs "\n")))))))

          ((equal? k "property-drawer")
           (let* ((props (filter (lambda (r) (equal? (.get r kind) "node-property"))
                                 (or (hash-get by-parent (.get rec id)) '())))
                  (rows (filter-map
                          (lambda (p)
                            (let ((pk (safe-field field-of p "key"))
                                  (pv (safe-field field-of p "value")))
                              (if (and (string? pk) (string? pv))
                                (string-append pk "|" (sanitize-inline pv))
                                #f)))
                          props)))
             (unless (null? rows)
               (emit! "drawer" "PROPERTIES" "" rec
                      (list (cons "drawer_name" "PROPERTIES")
                            (cons "drawer_rows" (str-join rows "\n")))))))

          ((equal? k "table")
           (let* ((tbl (extract-org-table bytes by-parent rec))
                  (hdr (car tbl))
                  (rows (cdr tbl)))
             (unless (and (string=? hdr "") (string=? rows ""))
               (emit! "table" "Table" "" rec
                      (list (cons "headers" hdr)
                            (cons "rows" rows))))))

          ((equal? k "plain-list")
           (let ((encoded (extract-org-list bytes by-parent field-of rec)))
             (unless (string=? encoded "")
               (emit! "list" "List" "" rec
                      (list (cons "items" encoded))))))

          ((equal? k "src-block")
           (let* ((lang (or (safe-field field-of rec "language") "text"))
                  (raw (slice-utf8-bytes bytes (.get rec start) (.get rec end)))
                  (code (strip-src-block-wrapper raw)))
             (emit! "code" lang code rec
                    (list (cons "lang" lang)))))

          ((equal? k "quote-block")
           (let* ((raw (slice-utf8-bytes bytes (.get rec start) (.get rec end)))
                  (body (str-trim (strip-src-block-wrapper raw))))
             (unless (string=? body "")
               (emit! "quote" "Quote" body rec '()))))

          ((equal? k "horizontal-rule")
           (emit! "rule" "---" "" rec '()))

          ((equal? k "paragraph")
           (let* ((raw (str-trim (slice-utf8-bytes bytes (.get rec start) (.get rec end))))
                  (spans (encode-inline-spans bytes by-parent rec)))
             (unless (string=? raw "")
               (emit! "paragraph" "" raw rec
                      (if (string=? spans "") '() (list (cons "spans" spans)))))))

          (else (void)))))

    (when root (visit root))
    (reverse out)))

;; Fast Org Imenu heading outline (`org:headings@<path>`).
(def (org-project-headings path source)
  (let* ((projected (marlin-context-org-source-graph source))
         (owned (.get projected source))
         (bytes (string->bytes owned))
         (g (.get projected graph))
         (field-of (.get g field-of))
         (headlines (sort-list (filter (lambda (r) (equal? (.get r kind) "headline"))
                                       (.get g records))
                               (lambda (a b) (< (.get a start) (.get b start))))))
    (let loop ((rem headlines) (i 0) (acc '()))
      (if (null? rem)
        (reverse acc)
        (let* ((r (car rem))
               (lvl (or (safe-field field-of r "level") "1"))
               (title (or (safe-field field-of r "title") ""))
               (todo (or (safe-field field-of r "todo-keyword") ""))
               (line (byte-offset->line bytes (.get r start)))
               (rec (make-bridge-record-vec
                      (string-append "org:heading:" (number->string i))
                      "org.heading"
                      title
                      todo
                      path
                      line
                      line
                      (list (cons "level" lvl) (cons "todo_word" todo)))))
          (loop (cdr rem) (+ i 1) (cons rec acc)))))))

;; Extract TODO/NEXT/WAITING/DONE agenda items (`org:agenda@<path>`).
(def (org-project-agenda path source)
  (let* ((projected (marlin-context-org-source-graph source))
         (owned (.get projected source))
         (bytes (string->bytes owned))
         (g (.get projected graph))
         (field-of (.get g field-of))
         (headlines (sort-list (filter (lambda (r)
                                         (and (equal? (.get r kind) "headline")
                                              (let ((tw (safe-field field-of r "todo-keyword")))
                                                (and (string? tw) (not (string=? tw ""))))))
                                       (.get g records))
                               (lambda (a b) (< (.get a start) (.get b start))))))
    (let loop ((rem headlines) (i 0) (acc '()))
      (if (null? rem)
        (reverse acc)
        (let* ((r (car rem))
               (todo (or (safe-field field-of r "todo-keyword") ""))
               (prio (or (safe-field field-of r "priority") ""))
               (title (or (safe-field field-of r "title") ""))
               (tags (as-string-list (safe-field field-of r "tags")))
               (line (byte-offset->line bytes (.get r start)))
               (rec (make-bridge-record-vec
                      (string-append "org:agenda:" (number->string i))
                      "org.agenda_item"
                      title
                      todo
                      path
                      line
                      line
                      (list (cons "todo_word" todo)
                            (cons "priority" prio)
                            (cons "tags" (str-join tags ","))))))
          (loop (cdr rem) (+ i 1) (cons rec acc)))))))

(def (org-handle-query verb arg payload)
  (cond
    ((string=? verb "headings") (org-project-headings arg payload))
    ((string=? verb "agenda") (org-project-agenda arg payload))
    (else (org-project-blocks arg payload))))

(def OrgBridgeModule
  (.o namespace: "org"
      owner-id: "marlin.scheme.org"
      description: "Authoritative Gerbil Scheme Org Mode parser & projector (orgize)"
      handle-query: org-handle-query))
