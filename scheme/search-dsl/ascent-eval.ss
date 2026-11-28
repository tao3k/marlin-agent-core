;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Zero-Eval Closed `:gerbil-ascent` Pruning, Derived Path/Name EDB
;;; Materialization, Anti-Mode-Collapse Sampling & Session Evaluation
;;; (`scheme/search-dsl/ascent-eval.ss`).
(import (only-in :clan/poo/object .get .ref)
        (only-in :std/string/misc string-contains)
        (only-in :gerbil-ascent/rules/closed-source ascent-closed-dsl-parts)
        (only-in :gerbil-ascent/program/api
                 gerbil-ascent-program
                 gerbil-ascent-open-session
                 gerbil-ascent-session-replace-source!
                 gerbil-ascent-session-run)
        (only-in :marlin-agent-core/repair/funcs format-canonical-search-form)
        "./prefilter.ss")
(export prune-ascent-parts-for-query
        path-is-test-file?
        path-is-hot-path-module?
        select-diverse-rows-up-to
        compile-closed-ascent-session
        run-compiled-ascent-session
        evaluate-closed-ascent-over-edb)

(def (string-contains-any? haystack needles)
  (let ((down (string-downcase haystack)))
    (let loop ((rest needles))
      (cond
        ((null? rest) #f)
        ((string-contains down (car rest)) #t)
        (else (loop (cdr rest)))))))

(def (path-is-test-file? path-str)
  (string-contains-any?
    path-str
    '("/tests/" "tests/" "/t/" "_test." "-test." "test_" "_mock." "mock" "fixture")))

(def (path-is-hot-path-module? path-str)
  (string-contains-any?
    path-str
    '("search-dsl" "edb_bridge" "converged_resident_index" "c-abi"
      "span_overlay" "linked_ui" "hot_path" "hot-path" "kernel")))

;;; Prune `fragments` to ONLY the transitive dependency cone of `query-part`,
;;; returning `(values pruned-program active-edb-names)`.
(def (prune-ascent-parts-for-query fragments query-part)
  (let* ((all-parts (append fragments (list query-part)))
         (all-rules (apply append (map (lambda (p) (.ref p 'rules)) all-parts)))
         (needed (reachable-relation-names query-part all-rules))
         (pruned-rules
           (filter (lambda (rule)
                     (ormap (lambda (h) (hash-get needed h))
                            (rule-head-relations rule)))
                   all-rules))
         (relations (make-hash-table-eq))
         (order '()))
    (for-each
      (lambda (part)
        (for-each
          (lambda (rel)
            (let* ((name (.ref rel 'name))
                   (keep? (or (hash-get needed name) (memq name +edb-relation-names+))))
              (when keep?
                (let ((old (hash-get relations name)))
                  (when (and old (not (= (.ref old 'arity) (.ref rel 'arity))))
                    (error "incompatible ASCENT relation arity across fragments" name))
                  (unless old
                    (hash-put! relations name rel)
                    (set! order (cons name order)))))))
          (.ref part 'relations)))
      all-parts)
    (let* ((ordered (map (lambda (n) (hash-ref relations n)) (reverse order)))
           (active-edb (filter (lambda (n) (hash-get needed n)) +edb-relation-names+))
           (program (gerbil-ascent-program
                      ordered pruned-rules 262144 262144 262144 +edb-relation-names+)))
      (values program active-edb))))

(def (select-query-output-relation query-part)
  (let* ((names (map (lambda (r) (.ref r 'name)) (.ref query-part 'relations)))
         (derived (filter (lambda (n) (not (memq n +edb-relation-names+))) names)))
    (cond
      ((memq 'answer derived) 'answer)
      ((pair? derived) (car (reverse derived)))
      (else (error "ASCENT query must declare at least one derived output relation")))))

(def (take-rows-up-to rows limit)
  (if (not limit)
    rows
    (let loop ((rest rows) (n limit) (acc '()))
      (if (or (null? rest) (<= n 0))
        (reverse acc)
        (loop (cdr rest) (- n 1) (cons (car rest) acc))))))

(def +boilerplate-symbol-keys+
  '("new" "default" "fmt" "from" "into" "drop" "as_str" "clone" "eq" "ne"
    "cmp" "partial_cmp" "hash" "deref" "deref_mut" "as_ref" "borrow"
    "len" "is_empty" "id" "name" "value" "build" "run" "parse" "next"
    "try_from" "try_into" "display" "to_string" "main" "init" "get" "set"))

(def (boilerplate-symbol-key? k)
  (and (string? k)
       (or (member k +boilerplate-symbol-keys+)
           (< (string-length k) 3))))

(def (collision-pair-row? row)
  (and (pair? row)
       (= (length row) 5)
       (string? (car row))
       (string? (cadr row))
       (integer? (caddr row))
       (string? (cadddr row))
       (integer? (list-ref row 4))))

(def (typed-collision-pair-row? row)
  (and (pair? row)
       (= (length row) 6)
       (string? (car row))
       (string? (cadr row))
       (string? (caddr row))
       (integer? (cadddr row))
       (string? (list-ref row 4))
       (integer? (list-ref row 5))))

(def (impact-scoped-row? row)
  (and (pair? row)
       (= (length row) 5)
       (string? (car row))
       (member (cadr row) '("prod" "test"))
       (integer? (list-ref row 4))))

(def (symbol-locator-row? row)
  (and (pair? row)
       (= (length row) 3)
       (string? (car row))
       (string? (cadr row))
       (integer? (caddr row))))

(def (round-robin-diverse-by-key rows limit)
  (let ((buckets (make-hash-table))
        (order '()))
    (for-each
      (lambda (row)
        (let* ((k (car row))
               (prev (hash-get buckets k)))
          (if prev
            (hash-put! buckets k (cons row prev))
            (begin
              (hash-put! buckets k (list row))
              (set! order (cons k order))))))
      rows)
    (let* ((rev-order (reverse order))
           (domain-keys (filter (lambda (k) (not (boilerplate-symbol-key? k))) rev-order))
           (boiler-keys (filter boilerplate-symbol-key? rev-order))
           (prioritized (append domain-keys boiler-keys))
           (queues (map (lambda (k) (reverse (hash-ref buckets k))) prioritized)))
      (let loop ((cur queues) (next '()) (rem limit) (acc '()))
        (cond
          ((<= rem 0) (reverse acc))
          ((null? cur)
           (if (null? next)
             (reverse acc)
             (loop (reverse next) '() rem acc)))
          (else
           (let ((q (car cur)))
             (loop (cdr cur)
                   (if (pair? (cdr q)) (cons (cdr q) next) next)
                   (- rem 1)
                   (cons (car q) acc)))))))))

;;; Compact 6-column multi-kind collision pairs `(kind name p1 l1 p2 l2)`
;;; giving every distinct kind fair representation (so high-volume `fn`
;;; collisions never starve scarce `struct` collisions) while compacting
;;; each kind into domain-prioritized round-robin spanning-tree witnesses.
(def (compact-typed-collision-rows rows limit)
  (let ((by-kind (make-hash-table))
        (order '()))
    (for-each
      (lambda (row)
        (let* ((k (car row))
               (tail (cdr row))
               (prev (hash-get by-kind k)))
          (if prev
            (hash-put! by-kind k (cons tail prev))
            (begin
              (hash-put! by-kind k (list tail))
              (set! order (cons k order))))))
      rows)
    (let* ((kinds (reverse order))
           (num-kinds (max 1 (length kinds)))
           (max-rows (or limit 1000000))
           (per-kind (max 6 (quotient (+ max-rows (- num-kinds 1)) num-kinds)))
           (queues
             (map (lambda (k)
                    (map (lambda (r) (cons k r))
                         (compact-collision-rows (reverse (hash-ref by-kind k)) per-kind)))
                  kinds)))
      (let loop ((cur queues) (next '()) (rem max-rows) (acc '()))
        (cond
          ((<= rem 0) (reverse acc))
          ((null? cur)
           (if (null? next)
             (reverse acc)
             (loop (reverse next) '() rem acc)))
          (else
           (let ((q (car cur)))
             (if (null? q)
               (loop (cdr cur) next rem acc)
               (loop (cdr cur)
                     (if (pair? (cdr q)) (cons (cdr q) next) next)
                     (- rem 1)
                     (cons (car q) acc))))))))))

;;; Compact quadratic `(name p1 l1 p2 l2)` collision pairs into spanning-tree
;;; chains over unique `(path . line)` sites, ranking domain-specific symbols
;;; ahead of boilerplate trait/stdlib names and by distinct-file count desc.
(def (compact-collision-rows rows limit)
  (let ((sites-by-name (make-hash-table)))
    (for-each
      (lambda (row)
        (let* ((name (car row))
               (s1 (cons (cadr row) (caddr row)))
               (s2 (cons (cadddr row) (list-ref row 4)))
               (tbl (or (hash-get sites-by-name name)
                        (let ((t (make-hash-table)))
                          (hash-put! sites-by-name name t)
                          t))))
          (hash-put! tbl s1 #t)
          (hash-put! tbl s2 #t)))
      rows)
    (let* ((entries
             (map (lambda (kv)
                    (let* ((name (car kv))
                           (sites (list-sort
                                    (lambda (a b)
                                      (or (string<? (car a) (car b))
                                          (and (string=? (car a) (car b))
                                               (< (cdr a) (cdr b)))))
                                    (hash-keys (cdr kv)))))
                      (list name (boilerplate-symbol-key? name) (length sites) sites)))
                  (hash->list sites-by-name)))
           (ranked
             (list-sort
               (lambda (a b)
                 (let ((ab (cadr a)) (bb (cadr b))
                       (ac (caddr a)) (bc (caddr b)))
                   (cond
                     ((and (not ab) bb) #t)
                     ((and ab (not bb)) #f)
                     ((> ac bc) #t)
                     ((< ac bc) #f)
                     (else (string<? (car a) (car b))))))
               entries))
           (queues
             (filter-map
               (lambda (entry)
                 (let ((name (car entry))
                       (sites (cadddr entry)))
                   (and (>= (length sites) 2)
                        (let ((anchor (car sites))
                              (peers (take-rows-up-to (cdr sites) 5)))
                          (map (lambda (peer)
                                 (list name (car anchor) (cdr anchor) (car peer) (cdr peer)))
                               peers)))))
               ranked))
           (max-rows (or limit 1000000)))
      ;; Round-robin across spanning-tree queues preserves MAP-Elites anti-mode-collapse
      ;; (every distinct symbol gets its first witness pair before any symbol takes a 2nd)
      ;; while ranking domain-specific symbols ahead of boilerplate names.
      (let loop ((cur queues) (next '()) (rem max-rows) (acc '()))
        (cond
          ((<= rem 0) (reverse acc))
          ((null? cur)
           (if (null? next)
             (reverse acc)
             (loop (reverse next) '() rem acc)))
          (else
           (let ((q (car cur)))
             (loop (cdr cur)
                   (if (pair? (cdr q)) (cons (cdr q) next) next)
                   (- rem 1)
                   (cons (car q) acc)))))))))

;;; Anti-mode-collapse Quality-Diversity / MAP-Elites niche round-robin selector:
;;; - For 5-column collision pairs `(name p1 l1 p2 l2)`, deduplicates quadratic
;;;   pairs into domain-prioritized spanning-tree chains with full site witnesses.
;;; - For 5-column `find-impact` rows `(role prod|test sym path line)`, preserves
;;;   ALL production rows ahead of test-only rows so test suites never starve prod.
;;; - Otherwise partitions by `(car row)` with domain keys before boilerplate keys.
(def (select-diverse-rows-up-to rows limit)
  (cond
    ((null? rows) rows)
    ((typed-collision-pair-row? (car rows))
     (compact-typed-collision-rows rows limit))
    ((and (collision-pair-row? (car rows))
          (or (and limit (> (length rows) limit))
              (> (length rows) 8)))
     (compact-collision-rows rows limit))
    ((or (not limit) (<= (length rows) limit)) rows)
    ((impact-scoped-row? (car rows))
     (let* ((prod-rows (filter (lambda (r) (equal? (cadr r) "prod")) rows))
            (test-rows (filter (lambda (r) (not (equal? (cadr r) "prod"))) rows))
            (ordered (append prod-rows test-rows)))
       (take-rows-up-to ordered limit)))
    ((or (not (pair? (car rows))) (< (length (car rows)) 2))
     (take-rows-up-to rows limit))
    ((symbol-locator-row? (car rows))
     (let* ((prod-rows (filter (lambda (r) (not (path-is-test-file? (cadr r)))) rows))
            (test-rows (filter (lambda (r) (path-is-test-file? (cadr r))) rows))
            (picked-prod (round-robin-diverse-by-key prod-rows limit))
            (rem (- limit (length picked-prod))))
       (if (or (<= rem 0) (null? test-rows))
         picked-prod
         (append picked-prod (round-robin-diverse-by-key test-rows rem)))))
    (else
     (round-robin-diverse-by-key rows limit))))

;;; Extract literal arguments referenced by derived path and name EDB relations
;;; anywhere in `form` (including inside `(not (...))`):
;;;   #(path-prefixes path-needles name-needles name-prefixes)
(def (extract-derived-pred-literals form)
  (let ((path-prefixes '())
        (path-needles '())
        (name-needles '())
        (name-prefixes '()))
    (def (visit x)
      (when (pair? x)
        (cond
          ((and (eq? (car x) 'path_has_prefix) (= (length x) 3) (string? (caddr x)))
           (set! path-prefixes (append-unique path-prefixes (caddr x))))
          ((and (eq? (car x) 'path_contains) (= (length x) 3) (string? (caddr x)))
           (set! path-needles (append-unique path-needles (caddr x))))
          ((and (eq? (car x) 'name_contains) (= (length x) 3) (string? (caddr x)))
           (set! name-needles (append-unique name-needles (caddr x))))
          ((and (eq? (car x) 'name_has_prefix) (= (length x) 3) (string? (caddr x)))
           (set! name-prefixes (append-unique name-prefixes (caddr x))))
          (else
           (visit (car x))
           (visit (cdr x))))))
    (visit form)
    (vector path-prefixes path-needles name-needles name-prefixes)))

(def (collect-edb-unique-paths edb)
  (let ((seen (make-hash-table))
        (out '()))
    (for-each
      (lambda (f)
        (when (and (pair? f) (>= (length f) 3) (string? (caddr f)))
          (let ((p (caddr f)))
            (unless (hash-get seen p)
              (hash-put! seen p #t)
              (set! out (cons p out))))))
      (edb-table-ref edb 'file))
    (for-each
      (lambda (e)
        (when (and (pair? e) (>= (length e) 4) (string? (list-ref e 3)))
          (let ((p (list-ref e 3)))
            (unless (hash-get seen p)
              (hash-put! seen p #t)
              (set! out (cons p out))))))
      (edb-table-ref edb 'edge))
    (for-each
      (lambda (r)
        (when (and (pair? r) (>= (length r) 2) (string? (cadr r)))
          (let ((p (cadr r)))
            (unless (hash-get seen p)
              (hash-put! seen p #t)
              (set! out (cons p out))))))
      (edb-table-ref edb 'symbol_ref))
    (reverse out)))

(def (collect-edb-unique-names edb)
  (let ((seen (make-hash-table))
        (out '()))
    (for-each
      (lambda (ent)
        (when (and (pair? ent) (>= (length ent) 6) (string? (list-ref ent 5)))
          (let ((nm (list-ref ent 5)))
            (unless (hash-get seen nm)
              (hash-put! seen nm #t)
              (set! out (cons nm out))))))
      (edb-table-ref edb 'entity))
    (reverse out)))

(def (materialize-derived-path-edb-rows rel-name all-paths pred-lits)
  (case rel-name
    ((is_test_path)
     (map list (filter path-is-test-file? all-paths)))
    ((prod_path)
     (map list (filter (lambda (p) (not (path-is-test-file? p))) all-paths)))
    ((path_has_prefix)
     (let ((prefixes (if (vector? pred-lits) (vector-ref pred-lits 0) '())))
       (apply append
         (map (lambda (pfx)
                (filter-map (lambda (p) (and (string-prefix? pfx p) (list p pfx))) all-paths))
              prefixes))))
    ((path_contains)
     (let ((needles (if (vector? pred-lits) (vector-ref pred-lits 1) '())))
       (apply append
         (map (lambda (needle)
                (filter-map (lambda (p) (and (string-contains p needle) (list p needle))) all-paths))
              needles))))
    (else '())))

;;; Orthography-normalized identifier key: lowercases and strips `_` and `-`
;;; so `"read-only"`, `"read_only"`, `"ReadOnly"`, and `"readonly"` match
;;; uniformly across Rust snake_case, CamelCase, and Gerbil Scheme kebab-case.
(def (normalize-ident-key s)
  (let* ((down (string-downcase s))
         (n (string-length down)))
    (call-with-output-string
      (lambda (port)
        (let loop ((i 0))
          (when (< i n)
            (let ((ch (string-ref down i)))
              (unless (or (char=? ch #\_) (char=? ch #\-))
                (write-char ch port)))
            (loop (+ i 1))))))))

(def (materialize-derived-name-edb-rows rel-name all-names pred-lits)
  (case rel-name
    ((name_contains)
     (let ((needles (if (vector? pred-lits) (vector-ref pred-lits 2) '())))
       (apply append
         (map (lambda (needle)
                (let* ((needle-lc (string-downcase needle))
                       (needle-norm (normalize-ident-key needle))
                       (use-norm? (and (>= (string-length needle-norm) 3)
                                       (not (string=? needle-norm needle-lc)))))
                  (filter-map
                    (lambda (nm)
                      (let ((nm-lc (string-downcase nm)))
                        (and (or (string-contains nm-lc needle-lc)
                                 (and (>= (string-length needle-norm) 3)
                                      (or use-norm?
                                          (string-contains nm-lc "_")
                                          (string-contains nm-lc "-"))
                                      (string-contains (normalize-ident-key nm-lc) needle-norm)))
                             (list nm needle))))
                    all-names)))
              needles))))
    ((name_has_prefix)
     (let ((prefixes (if (vector? pred-lits) (vector-ref pred-lits 3) '())))
       (apply append
         (map (lambda (pfx)
                (let* ((pfx-lc (string-downcase pfx))
                       (pfx-norm (normalize-ident-key pfx))
                       (use-norm? (and (>= (string-length pfx-norm) 3)
                                       (not (string=? pfx-norm pfx-lc)))))
                  (filter-map
                    (lambda (nm)
                      (let ((nm-lc (string-downcase nm)))
                        (and (or (string-prefix? pfx-lc nm-lc)
                                 (and (>= (string-length pfx-norm) 3)
                                      (or use-norm?
                                          (string-contains nm-lc "_")
                                          (string-contains nm-lc "-"))
                                      (string-prefix? pfx-norm (normalize-ident-key nm-lc))))
                             (list nm pfx))))
                    all-names)))
              prefixes))))
    (else '())))

;;; Compile a repaired closed `(ascent ...)` / `(ascent-fragment ...)` form into
;;; a reusable pruned `:gerbil-ascent` session:
;;;   #(compiled-ascent-session session target-rel repairs active-edb-names row-limit canonical-query context-lines pred-lits)
(def (compile-closed-ascent-session form registry repairs)
  (let-values (((pure-form row-limit context-lines) (strip-ascent-directives form)))
    (let ((query-parts (ascent-closed-dsl-parts (list pure-form)))
          (canonical-query (and (pair? repairs) (format-canonical-search-form form)))
          (pred-lits (extract-derived-pred-literals pure-form)))
      (unless (and (pair? query-parts) (null? (cdr query-parts)))
        (error "ASCENT query rejected: only closed inert :gerbil-ascent declarations are admitted (explicit column predicates and closed guards; host callbacks forbidden)" pure-form))
      (let* ((query-part (car query-parts))
             (target-rel (select-query-output-relation query-part)))
        (let-values (((program active-edb)
                      (prune-ascent-parts-for-query (.get registry ascent-fragments) query-part)))
          (let ((session (gerbil-ascent-open-session program)))
            (gerbil-ascent-session-run session)
            (vector 'compiled-ascent-session session target-rel repairs active-edb row-limit canonical-query context-lines pred-lits)))))))

;;; Execute a compiled pruned `:gerbil-ascent` session against an EDB alist,
;;; updating ONLY the active EDB relations actually referenced by the pruned cone.
(def (run-compiled-ascent-session compiled edb)
  (let* ((session (vector-ref compiled 1))
         (target-rel (vector-ref compiled 2))
         (repairs (vector-ref compiled 3))
         (active-edb (vector-ref compiled 4))
         (row-limit (and (> (vector-length compiled) 5) (vector-ref compiled 5)))
         (canonical-query (and (> (vector-length compiled) 6) (vector-ref compiled 6)))
         (context-lines (and (> (vector-length compiled) 7) (vector-ref compiled 7)))
         (pred-lits (and (> (vector-length compiled) 8) (vector-ref compiled 8)))
         (needs-paths? (ormap (lambda (r) (memq r +derived-path-edb-names+)) active-edb))
         (needs-names? (ormap (lambda (r) (memq r +derived-name-edb-names+)) active-edb))
         (all-paths (if needs-paths? (collect-edb-unique-paths edb) '()))
         (all-names (if needs-names? (collect-edb-unique-names edb) '())))
    (for-each
      (lambda (rel-name)
        (let ((rows (cond
                      ((memq rel-name +derived-path-edb-names+)
                       (materialize-derived-path-edb-rows rel-name all-paths pred-lits))
                      ((memq rel-name +derived-name-edb-names+)
                       (materialize-derived-name-edb-rows rel-name all-names pred-lits))
                      (else (edb-table-ref edb rel-name)))))
          (gerbil-ascent-session-replace-source! session rel-name rows)))
      active-edb)
    (let* ((result (gerbil-ascent-session-run session))
           (all-rows ((.ref result 'rows-of) target-rel))
           (total-count (length all-rows))
           (shown-rows (select-diverse-rows-up-to all-rows row-limit)))
      (vector 'search-result 'ascent target-rel shown-rows repairs total-count canonical-query context-lines))))

(def (evaluate-closed-ascent-over-edb form edb registry repairs)
  (let ((compiled (compile-closed-ascent-session form registry repairs)))
    (run-compiled-ascent-session compiled edb)))
