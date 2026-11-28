;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Zero-Regex Native Open EDB Fact Extractor in Gerbil Scheme (`scheme/search-dsl/extract.ss`).
;;;
;;; Produces protocol-native `semantic-open-edb.v1` `(entity, edge)` tuples
;;; directly from real AST / lexical token streams — ZERO regular expressions:
;;;   1. Gerbil Scheme (`.ss`): uses Gerbil's own safe reader
;;;      (`readtable-eval-allowed? #f`, `readtable-sharing-allowed? #f`) with
;;;      exact 1-based line tracking to extract POO `.def` prototypes, `.mix`
;;;      edges, `def`/`defstruct`/`defclass`/`defmethod`, `defsyntax`/`defrules`,
;;;      `:gerbil-parser` `deflanguage`, `c-define` FFI bindings, and `import` edges.
;;;   2. OpenSysML v2 (`.sysml`): pure token-stream declaration parser extracting
;;;      `package`, `requirement <'ID'>`, `part [def]`, `state def`, `action def`,
;;;      `enum def`, `attribute def`, `dependency`, and `allocation` edges.
;;;   3. Quint (`.qnt`): pure token-stream declaration parser extracting `module`,
;;;      `action`, `val` invariants, `[pure] def`, and `import` edges.
(import (only-in :gerbil/runtime/gambit
                 current-readtable
                 readtable-eval-allowed?-set
                 readtable-sharing-allowed?-set))
(export extract-scheme-open-edb-facts
        extract-sysml-open-edb-facts
        extract-quint-open-edb-facts
        extract-open-edb-facts-batch)

(def (make-entity-row path line name lang kind language-kind attrs)
  (vector path line name lang kind language-kind "gerbil-parser"
          (list->vector (map (lambda (kv) (vector (car kv) (cdr kv))) attrs))))

(def (make-edge-row src relation dst path line)
  (vector src relation dst path line))

;;; ============================================================================
;;; 1. Gerbil Scheme (`.ss`) Native Safe-Reader AST Extractor (Zero Regex!)
;;; ============================================================================

;;; Scan top-level Scheme forms with exact 1-based start line numbers without
;;; losing leading `#` tokens (only skips `#!` on line 1).
(def (read-scheme-forms-with-lines source)
  (let* ((len (string-length source))
         (start-pos (if (and (>= len 2)
                             (char=? (string-ref source 0) #\#)
                             (char=? (string-ref source 1) #\!))
                      (let find-nl ((i 2))
                        (cond
                          ((>= i len) len)
                          ((char=? (string-ref source i) #\newline) (+ i 1))
                          (else (find-nl (+ i 1)))))
                      0))
         (start-line (if (> start-pos 0) 2 1))
         (slice (if (> start-pos 0) (substring source start-pos len) source)))
    (parameterize ((current-readtable
                    (readtable-sharing-allowed?-set
                     (readtable-eval-allowed?-set (current-readtable) #f) #f)))
      (call-with-input-string slice
        (lambda (port)
          (let ((forms '()))
            (let loop ((line start-line))
              (let skip ((ln line))
                (let ((ch (peek-char port)))
                  (cond
                    ((eof-object? ch) (reverse forms))
                    ((char=? ch #\newline)
                     (read-char port)
                     (skip (+ ln 1)))
                    ((char-whitespace? ch)
                     (read-char port)
                     (skip ln))
                    ((char=? ch #\;)
                     (let eat ()
                       (let ((c (read-char port)))
                         (cond
                           ((eof-object? c) (reverse forms))
                           ((char=? c #\newline) (skip (+ ln 1)))
                           (else (eat))))))
                    (else
                     (let* ((form-line ln)
                            (form (with-catch (lambda (_) #!eof) (lambda () (read port)))))
                       (if (eof-object? form)
                         (reverse forms)
                         (begin
                           (set! forms (cons (cons form-line form) forms))
                           (let ((port-ln (with-catch
                                            (lambda (_) form-line)
                                            (lambda () (+ (input-port-line port) (- start-line 1))))))
                             (loop (max form-line port-ln)))))))))))))))))

(def (extract-import-targets spec)
  (cond
    ((symbol? spec) (list (symbol->string spec)))
    ((string? spec) (list spec))
    ((and (pair? spec) (memq (car spec) '(only-in except-in rename-in prefix-in)) (pair? (cdr spec)))
     (extract-import-targets (cadr spec)))
    (else '())))

(def +scheme-call-ignore-heads+
  '(quote quasiquote unquote unquote-splicing lambda fn case-lambda
    let let* letrec letrec* let-values let*-values
    if cond case when unless and or not begin do
    set! def define defstruct defclass defmethod defsyntax defrules
    import export module begin-foreign .def .o .mix .get .ref .slot?
    parameterize with-catch values car cdr cadr caddr cdddr cons list vector
    null? pair? symbol? string? number? integer? exact-integer? boolean? eq? eqv? equal?
    + - * / = < <= > >=))

(def (collect-scheme-call-edges owner form path line edges-box seen)
  (when (pair? form)
    (let ((head (car form)))
      (cond
        ((eq? head 'quote) #f)
        ((and (symbol? head) (not (memq head +scheme-call-ignore-heads+)))
         (let ((callee (symbol->string head)))
           (unless (or (string=? callee owner) (hash-get seen callee))
             (hash-put! seen callee #t)
             (set-box! edges-box
               (cons (make-edge-row owner "calls" callee path line)
                     (unbox edges-box)))))
         (for-each
           (lambda (sub) (collect-scheme-call-edges owner sub path line edges-box seen))
           (cdr form)))
        (else
         (for-each
           (lambda (sub) (collect-scheme-call-edges owner sub path line edges-box seen))
           form))))))

(def (collect-mix-edges owner form path line edges-box)
  (when (pair? form)
    (when (and (eq? (car form) '.mix) (pair? (cdr form)))
      (for-each
        (lambda (parent)
          (when (symbol? parent)
            (set-box! edges-box
              (cons (make-edge-row owner "mixes" (symbol->string parent) path line)
                    (unbox edges-box)))))
        (cdr form)))
    (for-each
      (lambda (sub) (collect-mix-edges owner sub path line edges-box))
      form)))

(def (walk-scheme-form path line form entities-box edges-box)
  (when (and (pair? form) (symbol? (car form)))
    (let ((head (car form))
          (rest (cdr form)))
      (case head
        ((begin begin-foreign module)
         (for-each
           (lambda (sub) (walk-scheme-form path line sub entities-box edges-box))
           rest))
        ((import)
         (for-each
           (lambda (spec)
             (for-each
               (lambda (target)
                 (set-box! edges-box
                   (cons (make-edge-row path "imports" target path line)
                         (unbox edges-box))))
               (extract-import-targets spec)))
           rest))
        ((.def defsearch-language)
         (when (and (pair? rest) (symbol? (car rest)))
           (let ((name (symbol->string (car rest))))
             (set-box! entities-box
               (cons (make-entity-row path line name "scheme" "item" "poo_def"
                                      (list (cons "form" (symbol->string head))))
                     (unbox entities-box)))
             (collect-mix-edges name (cdr rest) path line edges-box))))
        ((def)
         (when (pair? rest)
           (let ((sig (car rest)))
             (cond
               ((and (pair? sig) (symbol? (car sig)))
                (let ((name (symbol->string (car sig))))
                  (set-box! entities-box
                    (cons (make-entity-row path line name "scheme" "function" "def_proc" '())
                          (unbox entities-box)))
                  (collect-mix-edges name (cdr rest) path line edges-box)
                  (collect-scheme-call-edges name (cdr rest) path line edges-box (make-hash-table))))
               ((symbol? sig)
                (let ((name (symbol->string sig)))
                  (set-box! entities-box
                    (cons (make-entity-row path line name "scheme" "constant" "def_val" '())
                          (unbox entities-box)))
                  (collect-mix-edges name (cdr rest) path line edges-box)))))))
        ((defstruct)
         (when (pair? rest)
           (let ((name-sym (if (pair? (car rest)) (caar rest) (car rest))))
             (when (symbol? name-sym)
               (set-box! entities-box
                 (cons (make-entity-row path line (symbol->string name-sym)
                                        "scheme" "type" "defstruct" '())
                       (unbox entities-box)))))))
        ((defclass)
         (when (pair? rest)
           (let ((name-sym (if (pair? (car rest)) (caar rest) (car rest))))
             (when (symbol? name-sym)
               (set-box! entities-box
                 (cons (make-entity-row path line (symbol->string name-sym)
                                        "scheme" "class" "defclass" '())
                       (unbox entities-box)))))))
        ((defmethod)
         (when (and (pair? rest) (pair? (car rest)))
           (let ((head-spec (car rest)))
             (when (symbol? (car head-spec))
               (set-box! entities-box
                 (cons (make-entity-row path line (symbol->string (car head-spec))
                                        "scheme" "method" "defmethod" '())
                       (unbox entities-box)))))))
        ((defsyntax defrules)
         (when (pair? rest)
           (let ((name-sym (if (pair? (car rest)) (caar rest) (car rest))))
             (when (symbol? name-sym)
               (set-box! entities-box
                 (cons (make-entity-row path line (symbol->string name-sym)
                                        "scheme" "macro" (symbol->string head) '())
                       (unbox entities-box)))))))
        ((deflanguage)
         (when (and (pair? rest) (symbol? (car rest)))
           (set-box! entities-box
             (cons (make-entity-row path line (symbol->string (car rest))
                                    "scheme" "item" "deflanguage" '())
                   (unbox entities-box)))))
        ((c-define)
         (when (and (>= (length rest) 4)
                    (pair? (car rest))
                    (symbol? (caar rest))
                    (string? (list-ref rest 3)))
           (let ((scheme-fn (symbol->string (caar rest)))
                 (c-sym (list-ref rest 3)))
             (set-box! entities-box
               (cons (make-entity-row path line c-sym "scheme" "function" "c_define"
                                      (list (cons "schemeFn" scheme-fn)))
                     (unbox entities-box)))
             (set-box! edges-box
               (cons (make-edge-row scheme-fn "ffi_binds" c-sym path line)
                     (unbox edges-box))))))))))

(def (infer-scheme-module-role path)
  (let* ((slash-pos
           (let loop ((i (- (string-length path) 1)))
             (cond ((< i 0) -1)
                   ((char=? (string-ref path i) #\/) i)
                   (else (loop (- i 1))))))
         (base (substring path (+ slash-pos 1) (string-length path))))
    (cond
      ((string=? base "interface.ss") "interface")
      ((string=? base "objects.ss") "objects")
      ((string=? base "funcs.ss") "funcs")
      ((string=? base "config.ss") "config")
      ((string=? base "types.ss") "types")
      ((string=? base "syntax.ss") "syntax")
      (else #f))))

(def (collect-scheme-export-names forms)
  (let ((exported (make-hash-table)))
    (for-each
      (lambda (entry)
        (let ((form (cdr entry)))
          (when (and (pair? form) (eq? (car form) 'export))
            (for-each
              (lambda (spec)
                (cond
                  ((symbol? spec)
                   (hash-put! exported (symbol->string spec) #t))
                  ((and (pair? spec) (eq? (car spec) 'rename-out))
                   (for-each
                     (lambda (pair-spec)
                       (when (and (pair? pair-spec) (symbol? (car pair-spec)))
                         (hash-put! exported (symbol->string (car pair-spec)) #t)))
                     (cdr spec)))))
              (cdr form)))))
      forms)
    exported))

(def (decorate-scheme-entity-attrs name base-attrs exported-tbl role)
  (let* ((a1 (if (and exported-tbl (hash-get exported-tbl name))
               (cons (cons "exported" "true") base-attrs)
               base-attrs))
         (a2 (if role
               (cons (cons "moduleRole" role) a1)
               a1)))
    a2))

;;; Extract native Open EDB `(values entities-list edges-list)` from a Gerbil Scheme file.
(def (extract-scheme-open-edb-facts path source)
  (let ((entities-box (box '()))
        (edges-box (box '()))
        (role (infer-scheme-module-role path)))
    (with-catch
      (lambda (_) #f)
      (lambda ()
        (let* ((forms (read-scheme-forms-with-lines source))
               (exported-tbl (collect-scheme-export-names forms)))
          (for-each
            (lambda (entry)
              (walk-scheme-form path (car entry) (cdr entry) entities-box edges-box))
            forms)
          (when (or role (> (hash-length exported-tbl) 0))
            (set-box! entities-box
              (map (lambda (row)
                     (let* ((name (vector-ref row 2))
                            (old-vec (vector-ref row 7))
                            (old-alist
                              (let loop ((i 0) (acc '()))
                                (if (= i (vector-length old-vec))
                                  (reverse acc)
                                  (let ((kv (vector-ref old-vec i)))
                                    (loop (+ i 1)
                                          (cons (cons (vector-ref kv 0) (vector-ref kv 1)) acc))))))
                            (new-alist (decorate-scheme-entity-attrs name old-alist exported-tbl role)))
                       (vector (vector-ref row 0)
                               (vector-ref row 1)
                               name
                               (vector-ref row 3)
                               (vector-ref row 4)
                               (vector-ref row 5)
                               (vector-ref row 6)
                               (list->vector (map (lambda (kv) (vector (car kv) (cdr kv))) new-alist)))))
                   (unbox entities-box)))))))
    (values (reverse (unbox entities-box)) (reverse (unbox edges-box)))))

;;; ============================================================================
;;; 2. Pure Lexical Tokenizer for SysML v2 (`.sysml`) & Quint (`.qnt`) (Zero Regex!)
;;; ============================================================================

(def (ident-start-char? ch)
  (or (char-alphabetic? ch) (char=? ch #\_)))

(def (ident-body-char? ch)
  (or (char-alphabetic? ch) (char-numeric? ch)
      (char=? ch #\_) (char=? ch #\.) (char=? ch #\-)))

;;; Tokenize C-style comment / identifier / angle-short-name `<'ID'>` / punctuation
;;; into a list of `(line kind text)` tuples (zero regexes).
(def (tokenize-model-source source)
  (let ((len (string-length source)))
    (let loop ((i 0) (line 1) (acc '()))
      (if (>= i len)
        (reverse acc)
        (let ((ch (string-ref source i)))
          (cond
            ((char=? ch #\newline) (loop (+ i 1) (+ line 1) acc))
            ((char-whitespace? ch) (loop (+ i 1) line acc))
            ;; Line comment `// ...`
            ((and (char=? ch #\/) (< (+ i 1) len) (char=? (string-ref source (+ i 1)) #\/))
             (let eat ((j (+ i 2)))
               (cond
                 ((>= j len) (reverse acc))
                 ((char=? (string-ref source j) #\newline) (loop (+ j 1) (+ line 1) acc))
                 (else (eat (+ j 1))))))
            ;; Block comment `/* ... */`
            ((and (char=? ch #\/) (< (+ i 1) len) (char=? (string-ref source (+ i 1)) #\*))
             (let eat ((j (+ i 2)) (ln line))
               (cond
                 ((>= (+ j 1) len) (reverse acc))
                 ((and (char=? (string-ref source j) #\*)
                       (char=? (string-ref source (+ j 1)) #\/))
                  (loop (+ j 2) ln acc))
                 ((char=? (string-ref source j) #\newline) (eat (+ j 1) (+ ln 1)))
                 (else (eat (+ j 1) ln)))))
            ;; SysML short name `<'MAR-SRCH-R01'>`
            ((and (char=? ch #\<) (< (+ i 1) len) (char=? (string-ref source (+ i 1)) #\'))
             (let eat ((j (+ i 2)))
               (cond
                 ((>= (+ j 1) len) (loop (+ i 1) line acc))
                 ((and (char=? (string-ref source j) #\')
                       (char=? (string-ref source (+ j 1)) #\>))
                  (loop (+ j 2) line
                        (cons (list line 'short-name (substring source (+ i 2) j)) acc)))
                 (else (eat (+ j 1))))))
            ;; Double-quoted string literal
            ((char=? ch #\")
             (let eat ((j (+ i 1)) (ln line))
               (cond
                 ((>= j len) (reverse acc))
                 ((char=? (string-ref source j) #\\) (eat (+ j 2) ln))
                 ((char=? (string-ref source j) #\")
                  (loop (+ j 1) ln
                        (cons (list line 'string (substring source (+ i 1) j)) acc)))
                 ((char=? (string-ref source j) #\newline) (eat (+ j 1) (+ ln 1)))
                 (else (eat (+ j 1) ln)))))
            ;; Identifier or keyword
            ((ident-start-char? ch)
             (let eat ((j (+ i 1)))
               (if (and (< j len) (ident-body-char? (string-ref source j)))
                 (eat (+ j 1))
                 (loop j line
                       (cons (list line 'ident (substring source i j)) acc)))))
            ;; Punctuation
            (else
             (loop (+ i 1) line (cons (list line 'punct (string ch)) acc)))))))))

(def (tok-line t) (car t))
(def (tok-kind t) (cadr t))
(def (tok-text t) (caddr t))

(def (extract-sysml-open-edb-facts path source)
  (let ((toks (tokenize-model-source source))
        (entities '())
        (edges '()))
    (let loop ((rest toks))
      (if (null? rest)
        (values (reverse entities) (reverse edges))
        (let* ((t0 (car rest))
               (ln (tok-line t0))
               (k0 (tok-kind t0))
               (w0 (tok-text t0))
               (r1 (cdr rest)))
          (cond
            ((not (eq? k0 'ident)) (loop r1))
            ((and (string=? w0 "package") (pair? r1) (eq? (tok-kind (car r1)) 'ident))
             (set! entities (cons (make-entity-row path ln (tok-text (car r1))
                                                   "sysml" "module" "package_def" '())
                                  entities))
             (loop (cdr r1)))
            ((and (string=? w0 "requirement") (pair? r1))
             (let* ((is-def? (and (eq? (tok-kind (car r1)) 'ident)
                                  (string=? (tok-text (car r1)) "def")))
                    (after-def (if is-def? (cdr r1) r1)))
               (cond
                 ((and (pair? after-def)
                       (eq? (tok-kind (car after-def)) 'short-name)
                       (pair? (cdr after-def))
                       (eq? (tok-kind (cadr after-def)) 'ident))
                  (set! entities
                    (cons (make-entity-row path ln (tok-text (cadr after-def))
                                           "sysml" "item" "requirement_def"
                                           (list (cons "reqId" (tok-text (car after-def)))))
                          entities))
                  (loop (cddr after-def)))
                 ((and (pair? after-def) (eq? (tok-kind (car after-def)) 'ident))
                  (set! entities
                    (cons (make-entity-row path ln (tok-text (car after-def))
                                           "sysml" "item" "requirement_def" '())
                          entities))
                  (loop (cdr after-def)))
                 (else (loop r1)))))
            ((and (or (string=? w0 "part") (string=? w0 "state")
                      (string=? w0 "action") (string=? w0 "enum")
                      (string=? w0 "attribute"))
                  (pair? r1))
             (let* ((is-def? (and (eq? (tok-kind (car r1)) 'ident)
                                  (string=? (tok-text (car r1)) "def")))
                    (after-def (if is-def? (cdr r1) r1))
                    (has-short? (and (pair? after-def)
                                     (eq? (tok-kind (car after-def)) 'short-name)))
                    (short-id (and has-short? (tok-text (car after-def))))
                    (name-rest (if has-short? (cdr after-def) after-def)))
               (if (and (pair? name-rest) (eq? (tok-kind (car name-rest)) 'ident))
                 (let* ((name (tok-text (car name-rest)))
                        (kind (cond
                                ((string=? w0 "action") "function")
                                ((string=? w0 "state") "item")
                                (else "type")))
                        (lkind (string-append w0 "_def"))
                        (attrs (if short-id (list (cons "shortId" short-id)) '())))
                   (set! entities
                     (cons (make-entity-row path ln name "sysml" kind lkind attrs)
                           entities))
                   (loop (cdr name-rest)))
                 (loop r1))))
            ((and (string=? w0 "dependency")
                  (>= (length r1) 5)
                  (eq? (tok-kind (car r1)) 'ident)
                  (string=? (tok-text (cadr r1)) "from")
                  (eq? (tok-kind (caddr r1)) 'ident)
                  (string=? (tok-text (cadddr r1)) "to")
                  (eq? (tok-kind (list-ref r1 4)) 'ident))
             (set! edges
               (cons (make-edge-row (tok-text (caddr r1)) "depends"
                                    (tok-text (list-ref r1 4)) path ln)
                     edges))
             (loop (list-tail r1 5)))
            ((and (string=? w0 "allocation")
                  (>= (length r1) 5)
                  (eq? (tok-kind (car r1)) 'ident)
                  (string=? (tok-text (cadr r1)) "allocate")
                  (eq? (tok-kind (caddr r1)) 'ident)
                  (string=? (tok-text (cadddr r1)) "to")
                  (eq? (tok-kind (list-ref r1 4)) 'ident))
             (set! edges
               (cons (make-edge-row (tok-text (caddr r1)) "allocates"
                                    (tok-text (list-ref r1 4)) path ln)
                     edges))
             (loop (list-tail r1 5)))
            (else (loop r1))))))))

(def (extract-quint-open-edb-facts path source)
  (let ((toks (tokenize-model-source source))
        (entities '())
        (edges '()))
    (let loop ((rest toks))
      (if (null? rest)
        (values (reverse entities) (reverse edges))
        (let* ((t0 (car rest))
               (ln (tok-line t0))
               (k0 (tok-kind t0))
               (w0 (tok-text t0))
               (r1 (cdr rest)))
          (cond
            ((not (eq? k0 'ident)) (loop r1))
            ((and (string=? w0 "module") (pair? r1) (eq? (tok-kind (car r1)) 'ident))
             (set! entities (cons (make-entity-row path ln (tok-text (car r1))
                                                   "quint" "module" "quint_module" '())
                                  entities))
             (loop (cdr r1)))
            ((and (string=? w0 "action") (pair? r1) (eq? (tok-kind (car r1)) 'ident))
             (set! entities (cons (make-entity-row path ln (tok-text (car r1))
                                                   "quint" "function" "quint_action" '())
                                  entities))
             (loop (cdr r1)))
            ((and (string=? w0 "val") (pair? r1) (eq? (tok-kind (car r1)) 'ident))
             (set! entities (cons (make-entity-row path ln (tok-text (car r1))
                                                   "quint" "item" "quint_invariant" '())
                                  entities))
             (loop (cdr r1)))
            ((and (string=? w0 "pure") (pair? r1) (string=? (tok-text (car r1)) "def")
                  (pair? (cdr r1)) (eq? (tok-kind (cadr r1)) 'ident))
             (set! entities (cons (make-entity-row path ln (tok-text (cadr r1))
                                                   "quint" "function" "quint_def" '())
                                  entities))
             (loop (cddr r1)))
            ((and (string=? w0 "def") (pair? r1) (eq? (tok-kind (car r1)) 'ident))
             (set! entities (cons (make-entity-row path ln (tok-text (car r1))
                                                   "quint" "function" "quint_def" '())
                                  entities))
             (loop (cdr r1)))
            ((and (string=? w0 "import") (pair? r1) (eq? (tok-kind (car r1)) 'ident))
             (set! edges (cons (make-edge-row path "imports" (tok-text (car r1)) path ln)
                               edges))
             (loop (cdr r1)))
            (else (loop r1))))))))

;;; Batch entrypoint invoked from Rust via rooted C-ABI:
;;;   input: `#(#(path lang body) ...)`
;;;   output: `#(entities-vec edges-vec)`
(def (extract-open-edb-facts-batch files-vec)
  (unless (vector? files-vec)
    (error "extract-open-edb-facts-batch requires a vector of #(path lang body)"))
  (let ((all-entities '())
        (all-edges '()))
    (for-each
      (lambda (entry)
        (when (and (vector? entry) (= (vector-length entry) 3))
          (let ((path (vector-ref entry 0))
                (lang (vector-ref entry 1))
                (body (vector-ref entry 2)))
            (let-values (((ents eds)
                          (cond
                            ((string=? lang "scheme") (extract-scheme-open-edb-facts path body))
                            ((string=? lang "sysml") (extract-sysml-open-edb-facts path body))
                            ((string=? lang "quint") (extract-quint-open-edb-facts path body))
                            (else (values '() '())))))
              (set! all-entities (append all-entities ents))
              (set! all-edges (append all-edges eds))))))
      (vector->list files-vec))
    (vector (list->vector all-entities) (list->vector all-edges))))
