;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Yuanxu JSON-Schema-Driven Copilot Helm TAB Completion Engine
;;; (`scheme/search-dsl/completions.ss`).
;;;
;;; Zero hardcoded keyword tables, zero hardcoded language lists, zero
;;; hardcoded AST kind tables, zero hardcoded workspace paths.
;;;
;;; Loads and normalizes any JSON Schema Draft 2020-12 protocol document
;;; (ASP universal request schemas, per-language `targetedSchema`, or gRPC
;;; wire JSON Schemas) directly through Yuanxu's `:tao3k/yuanxu/modules/json-schema`
;;; normalizer (`yuanxu-json-schema-normalize`) and projects every normalized
;;; `YuanxuJsonSchemaProperty` and `enum`/`const`/`boolean` value domain into
;;; typed Helm TAB completion rows.
(import (only-in :clan/poo/object .get .ref)
        (only-in :std/string/misc string-contains string-trim)
        (only-in :tao3k/yuanxu/modules/json-schema/json-schema-source
                 yuanxu-json-schema-read-file
                 yuanxu-json-schema-read-port)
        (only-in :tao3k/yuanxu/modules/json-schema/json-schema-normalize
                 +yuanxu-json-schema-missing+
                 yuanxu-json-schema-normalize)
        (only-in :tao3k/yuanxu/modules/json-schema/json-schema-ir
                 yuanxu-json-schema-normalization-schema
                 yuanxu-json-schema-node?
                 yuanxu-json-schema-node-kind
                 yuanxu-json-schema-node-value
                 yuanxu-json-schema-node-metadata
                 yuanxu-json-schema-object?
                 yuanxu-json-schema-object-properties
                 yuanxu-json-schema-property-name
                 yuanxu-json-schema-property-schema
                 yuanxu-json-schema-property-doc)
        "./syntax.ss"
        "./objects.ss"
        "./prefilter.ss")
(export search-dsl-completions
        introspect-relation-completions
        json-schema->completion-entries
        json-schema-file->completion-entries
        json-schema-string->completion-entries)

(def (strip-quotes s)
  (let* ((n (string-length s))
         (s1 (if (and (> n 0) (char=? (string-ref s 0) #\")) (substring s 1 n) s))
         (n1 (string-length s1)))
    (if (and (> n1 0) (char=? (string-ref s1 (- n1 1)) #\"))
      (substring s1 0 (- n1 1))
      s1)))

(def (keyword-token? tok)
  (let ((n (string-length tok)))
    (and (> n 1)
         (or (char=? (string-ref tok (- n 1)) #\:)
             (char=? (string-ref tok 0) #\:)))))

(def (normalize-kw-token tok)
  (let* ((lc (string-downcase tok))
         (n (string-length lc)))
    (cond
      ((and (> n 1) (char=? (string-ref lc (- n 1)) #\:))
       (substring lc 0 (- n 1)))
      ((and (> n 1) (char=? (string-ref lc 0) #\:))
       (substring lc 1 n))
      (else lc))))

(def (canonical-kw-key raw)
  (list->string
    (filter (lambda (ch) (not (or (char=? ch #\-) (char=? ch #\_))))
            (string->list (normalize-kw-token raw)))))

(def (tokenize-sexp-prefix text)
  (let ((len (string-length text)))
    (let loop ((i 0) (acc '()))
      (cond
        ((>= i len) (reverse acc))
        ((char-whitespace? (string-ref text i))
         (loop (+ i 1) acc))
        ((or (char=? (string-ref text i) #\()
             (char=? (string-ref text i) #\)))
         (loop (+ i 1) (cons (string (string-ref text i)) acc)))
        ((char=? (string-ref text i) #\")
         (let qloop ((j (+ i 1)))
           (cond
             ((>= j len) (reverse (cons (substring text i len) acc)))
             ((and (char=? (string-ref text j) #\\) (< (+ j 1) len))
              (qloop (+ j 2)))
             ((char=? (string-ref text j) #\")
              (loop (+ j 1) (cons (substring text i (+ j 1)) acc)))
             (else (qloop (+ j 1))))))
        (else
         (let tloop ((j (+ i 1)))
           (if (or (>= j len)
                   (char-whitespace? (string-ref text j))
                   (char=? (string-ref text j) #\()
                   (char=? (string-ref text j) #\))
                   (char=? (string-ref text j) #\"))
             (loop j (cons (substring text i j) acc))
             (tloop (+ j 1)))))))))

(def (last-token-fragment text)
  (let ((trimmed (string-trim text)))
    (if (string=? trimmed "")
      ""
      (let loop ((i (- (string-length trimmed) 1)))
        (if (< i 0)
          trimmed
          (let ((ch (string-ref trimmed i)))
            (if (or (char-whitespace? ch) (char=? ch #\() (char=? ch #\)) (char=? ch #\"))
              (substring trimmed (+ i 1) (string-length trimmed))
              (loop (- i 1)))))))))

(def (extract-active-lang tokens)
  (let loop ((rest tokens) (found #f))
    (cond
      ((null? rest) found)
      ((and (keyword-token? (car rest))
            (member (canonical-kw-key (car rest)) '("lang" "language" "languageid"))
            (pair? (cdr rest)))
       (let ((val (string-downcase (strip-quotes (cadr rest)))))
         (loop (cddr rest) (if (string=? val "") found val))))
      ((and (string=? (car rest) "use") (pair? (cdr rest)))
       (loop (cddr rest) (string-downcase (strip-quotes (cadr rest)))))
      (else (loop (cdr rest) found)))))

(def (extract-used-keywords tokens)
  (let loop ((rest tokens) (acc '()))
    (if (null? rest)
      acc
      (if (keyword-token? (car rest))
        (let ((kw (canonical-kw-key (car rest))))
          (loop (cdr rest) (if (member kw acc) acc (cons kw acc))))
        (loop (cdr rest) acc)))))

;;; Format a scalar JSON Schema enum/const/boolean value for DSL insertion.
(def (format-schema-scalar val)
  (cond
    ((string? val) (string-append "\"" val "\""))
    ((boolean? val) (if val "#t" "#f"))
    ((number? val) (number->string val))
    ((symbol? val) (symbol->string val))
    (else #f)))

(def (append-unique-scalars existing incoming)
  (let loop ((rest incoming) (acc existing))
    (cond
      ((null? rest) acc)
      ((member (car rest) acc) (loop (cdr rest) acc))
      (else (loop (cdr rest) (append acc (list (car rest))))))))

;;; Extract all allowed scalar values from a normalized Yuanxu JSON Schema node
;;; (`enum`, `const`, `boolean`, `all-of`, `any-of`, `one-of`).
(def (extract-node-scalar-values node)
  (if (not (yuanxu-json-schema-node? node))
    '()
    (let* ((k (yuanxu-json-schema-node-kind node))
           (v (yuanxu-json-schema-node-value node))
           (meta (yuanxu-json-schema-node-metadata node))
           (const-pair (and (list? meta) (assq 'const meta)))
           (const-vals (if (and const-pair (not (eq? (cdr const-pair) +yuanxu-json-schema-missing+)))
                         (list (cdr const-pair))
                         '()))
           (enum-pair (and (list? meta) (assq 'enum meta)))
           (meta-enum-vals (if (and enum-pair (list? (cdr enum-pair)))
                             (cdr enum-pair)
                             '()))
           (kind-vals
             (case k
               ((enum) (if (list? v) v '()))
               ((boolean) '(#t #f))
               ((all-of any-of one-of)
                (if (list? v)
                  (apply append (map extract-node-scalar-values v))
                  '()))
               (else '()))))
      (append-unique-scalars
        (append-unique-scalars const-vals meta-enum-vals)
        kind-vals))))

(def (extract-node-type-label node)
  (if (not (yuanxu-json-schema-node? node))
    "any"
    (let ((k (yuanxu-json-schema-node-kind node))
          (v (yuanxu-json-schema-node-value node)))
      (case k
        ((all-of any-of one-of)
         (if (list? v)
           (let ((non-null (filter (lambda (n)
                                     (and (yuanxu-json-schema-node? n)
                                          (not (eq? (yuanxu-json-schema-node-kind n) 'null))))
                                   v)))
             (if (pair? non-null)
               (extract-node-type-label (car non-null))
               (symbol->string k)))
           (symbol->string k)))
        (else (symbol->string k))))))

(def (extract-object-properties-from-node node)
  (if (not (yuanxu-json-schema-node? node))
    '()
    (let ((k (yuanxu-json-schema-node-kind node))
          (v (yuanxu-json-schema-node-value node)))
      (cond
        ((and (eq? k 'object) (yuanxu-json-schema-object? v))
         (yuanxu-json-schema-object-properties v))
        ((and (memq k '(all-of any-of one-of)) (list? v))
         (apply append (map extract-object-properties-from-node v)))
        (else '())))))

;;; Pure generic Yuanxu JSON Schema IR -> Helm TAB completion projector.
;;; Works identically on any ASP universal protocol schema, language
;;; `targetedSchema`, or gRPC wire JSON Schema normalized by Yuanxu.
(def (json-schema->completion-entries owner-scope schema-datum (doc-table #f))
  (let* ((norm (yuanxu-json-schema-normalize schema-datum))
         (root-node (yuanxu-json-schema-normalization-schema norm))
         (props (extract-object-properties-from-node root-node))
         (seen-kws (make-hash-table))
         (entries '()))
    (for-each
      (lambda (prop)
        (let* ((raw-name (yuanxu-json-schema-property-name prop))
               (prop-name (if (symbol? raw-name) (symbol->string raw-name) raw-name))
               (kw-form (string-append prop-name ":"))
               (prop-schema (yuanxu-json-schema-property-schema prop))
               (type-label (extract-node-type-label prop-schema))
               (prop-doc (or (yuanxu-json-schema-property-doc prop)
                             (string-append "JSON Schema property `" prop-name "` (" type-label ")")))
               (scalars (extract-node-scalar-values prop-schema)))
          (unless (hash-get seen-kws kw-form)
            (hash-put! seen-kws kw-form #t)
            (set! entries
              (cons (list kw-form
                          "keyword"
                          (string-append "schema:" type-label)
                          owner-scope
                          prop-doc
                          (string-append kw-form " "))
                    entries)))
          (for-each
            (lambda (sc)
              (let ((rendered (format-schema-scalar sc)))
                (when rendered
                  (let ((vdoc (or (and doc-table (string? sc) (hash-get doc-table sc))
                                  (string-append "Schema value for `" prop-name "` (" owner-scope ")"))))
                    (set! entries
                      (cons (list rendered
                                  "value"
                                  kw-form
                                  owner-scope
                                  vdoc
                                  (string-append rendered " "))
                            entries))))))
            scalars)))
      props)
    (reverse entries)))

;;; Load any `.schema.json` file (e.g. ASP universal protocol schemas) through
;;; Yuanxu's `yuanxu-json-schema-read-file` and project completions.
(def (json-schema-file->completion-entries owner-scope path)
  (json-schema->completion-entries owner-scope (yuanxu-json-schema-read-file path)))

;;; Parse any JSON Schema string (e.g. forwarded over HTTP or gRPC) through
;;; Yuanxu's `yuanxu-json-schema-read-port` and project completions.
(def (json-schema-string->completion-entries owner-scope json-str)
  (call-with-input-string
    json-str
    (lambda (port)
      (json-schema->completion-entries owner-scope (yuanxu-json-schema-read-port port)))))

(def (unique-strings items)
  (let loop ((rest items) (acc '()))
    (cond
      ((null? rest) (reverse acc))
      ((member (car rest) acc) (loop (cdr rest) acc))
      (else (loop (cdr rest) (cons (car rest) acc))))))

(def (language-contract->json-schema contract)
  (let* ((provider-lang (list-ref contract 1))
         (kind-map (list-ref contract 4))
         (native-kinds (unique-strings (map car kind-map)))
         (general-kinds (unique-strings (map cadr kind-map))))
    `((type . "object")
      (additionalProperties . #f)
      (required . ("languageId" "kind" "languageKind" "name" "line"))
      (properties
       . ((languageId   . ((type . "string") (const . ,provider-lang)))
          (kind         . ((type . "string") (enum . ,general-kinds)))
          (languageKind . ((type . "string") (enum . ,native-kinds)))
          (name         . ((type . "string") (minLength . 1)))
          (line         . ((type . "integer") (minimum . 1))))))))

(def (contract-doc-table contract)
  (let ((tbl (make-hash-table)))
    (for-each
      (lambda (km)
        (let ((nk (list-ref km 0))
              (gk (list-ref km 1))
              (doc (list-ref km 2)))
          (when (and (string? doc) (> (string-length doc) 0))
            (unless (hash-get tbl nk) (hash-put! tbl nk doc))
            (unless (hash-get tbl gk) (hash-put! tbl gk doc)))))
      (list-ref contract 4))
    tbl))

(def (contract-extension-entries contract)
  (let ((lang-label (symbol->string (car contract))))
    (filter-map
      (lambda (ext)
        (let* ((n (string-length ext))
               (clean (if (and (> n 0) (char=? (string-ref ext 0) #\.))
                        (substring ext 1 n)
                        ext)))
          (and (> (string-length clean) 0)
               (list (string-append "\"" clean "\"")
                     "value"
                     "ext:"
                     lang-label
                     (string-append "Extension " ext " from " lang-label " manifest")
                     (string-append "\"" clean "\" ")))))
      (list-ref contract 2))))

(def (introspect-relation-completions registry)
  (let ((meta (.get registry relation-meta)))
    (map (lambda (rel)
           (let* ((name (.ref rel 'name))
                  (name-str (symbol->string name))
                  (sig (render-relation-signature rel))
                  (info (assq name meta))
                  (lang (if info (cadr info) "ascent"))
                  (doc (if info (list-ref info 4) "Derived :gerbil-ascent relation"))
                  (kind (if (memq name +edb-relation-names+) "edb" "relation")))
             (list name-str kind sig lang doc sig)))
         (collect-unique-relations (.get registry ascent-fragments)))))

(def (language-value-completions registry)
  (map (lambda (lang-sym)
         (let ((s (symbol->string lang-sym)))
           (list (string-append "\"" s "\"")
                 "value"
                 "lang:"
                 s
                 (string-append "Registered language `" s "`")
                 (string-append "\"" s "\" "))))
       (.get registry languages)))

(def (project-registry-schema-completions registry)
  (let* ((lang-entries
           (apply append
             (map (lambda (contract)
                    (let ((lang-label (symbol->string (car contract))))
                      (append
                        (json-schema->completion-entries
                          lang-label
                          (language-contract->json-schema contract)
                          (contract-doc-table contract))
                        (contract-extension-entries contract))))
                  (.get registry language-contracts))))
         (protocol-entries
           (apply append
             (map (lambda (pair)
                    (json-schema->completion-entries (car pair) (cdr pair)))
                  (.get registry protocol-json-schemas)))))
    (append (language-value-completions registry)
            protocol-entries
            lang-entries)))

(def (kw-targets-match? pending-canon sig-canon)
  (or (string=? pending-canon sig-canon)
      (and (member pending-canon '("lang" "language" "languageid"))
           (member sig-canon '("lang" "language" "languageid")))
      (and (member pending-canon '("kind" "kinds"))
           (member sig-canon '("kind" "kinds")))
      (and (member pending-canon '("ext" "extension"))
           (member sig-canon '("ext" "extension")))))

(def (filter-entries-by-needle entries needle)
  (if (string=? needle "")
    entries
    (let ((lc (string-downcase (strip-quotes needle))))
      (filter
        (lambda (entry)
          (or (string-contains (string-downcase (car entry)) lc)
              (string-contains (string-downcase (cadddr entry)) lc)
              (string-contains (string-downcase (list-ref entry 4)) lc)))
        entries))))

(def (dedup-entries entries)
  (let ((seen (make-hash-table))
        (out '()))
    (for-each
      (lambda (e)
        (let ((key (list (car e) (cadr e) (caddr e) (cadddr e))))
          (unless (hash-get seen key)
            (hash-put! seen key #t)
            (set! out (cons e out)))))
      entries)
    (reverse out)))

(def (search-dsl-completions input-text (registry SearchLanguageRegistryValue))
  (let* ((raw (if (string? input-text) input-text ""))
         (trimmed (string-trim raw))
         (trailing-space? (and (> (string-length raw) 0)
                               (char-whitespace? (string-ref raw (- (string-length raw) 1)))))
         (all-base (append (.get registry syntax-forms)
                           (introspect-relation-completions registry)))
         (schema-entries (dedup-entries (project-registry-schema-completions registry))))
    (cond
      ;; Empty preload (used by TUI startup to hydrate completion catalog with zero hardcoding):
      ((string=? trimmed "")
       (list->vector (map list->vector (append all-base schema-entries))))
      ;; Root open-paren `(` -> top-level forms & EDB entrypoints:
      ((string=? trimmed "(")
       (list->vector
         (map list->vector
              (filter (lambda (entry)
                        (member (cadr entry) '("ascent" "dusa" "parser" "edb")))
                      all-base))))
      (else
       (let* ((tokens (tokenize-sexp-prefix raw))
              (head (and (>= (length tokens) 2)
                         (string=? (car tokens) "(")
                         (string-downcase (cadr tokens))))
              (active-lang (extract-active-lang tokens))
              (used-kws (extract-used-keywords tokens))
              (rev-toks (reverse tokens))
              (last-tok (and (pair? rev-toks) (car rev-toks)))
              (prev-tok (and (pair? rev-toks) (pair? (cdr rev-toks)) (cadr rev-toks)))
              (pending-kw
                (cond
                  ((and trailing-space? last-tok (keyword-token? last-tok))
                   (canonical-kw-key last-tok))
                  ((and (not trailing-space?) prev-tok (keyword-token? prev-tok))
                   (canonical-kw-key prev-tok))
                  (else #f)))
              (needle (if trailing-space? "" (last-token-fragment trimmed)))
              (pending-values
                (if pending-kw
                  (filter
                    (lambda (e)
                      (and (string=? (cadr e) "value")
                           (kw-targets-match? pending-kw (canonical-kw-key (caddr e)))
                           (or (member pending-kw '("lang" "language" "languageid"))
                               (not active-lang)
                               (string=? (string-downcase (cadddr e)) active-lang)
                               (string=? (cadddr e) "*"))))
                    schema-entries)
                  '()))
              (context-keywords
                (filter
                  (lambda (e)
                    (and (string=? (cadr e) "keyword")
                         (not (member (canonical-kw-key (car e)) used-kws))
                         (or (and head (string=? (string-downcase (cadddr e)) head))
                             (and active-lang (string=? (string-downcase (cadddr e)) active-lang))
                             (string=? (cadddr e) "*"))))
                  schema-entries))
              (candidates
                (cond
                  ((and (not trailing-space?) (= (length tokens) 2) (string=? (car tokens) "("))
                   (filter-entries-by-needle
                     (filter (lambda (e) (member (cadr e) '("ascent" "dusa" "parser" "edb"))) all-base)
                     needle))
                  ((pair? pending-values)
                   (filter-entries-by-needle pending-values needle))
                  ((and active-lang (pair? context-keywords)
                        (or trailing-space? (string=? needle "")
                            (pair? (filter-entries-by-needle context-keywords needle))))
                   (filter-entries-by-needle context-keywords needle))
                  (else
                   (let ((matched (filter-entries-by-needle all-base needle)))
                     (if active-lang
                       (append
                         (filter (lambda (e) (string=? (string-downcase (cadddr e)) active-lang)) matched)
                         (filter (lambda (e) (not (string=? (string-downcase (cadddr e)) active-lang))) matched))
                       matched))))))
         (list->vector (map list->vector candidates)))))))
