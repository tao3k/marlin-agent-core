;; Minimal Gerbil command-adapter shim.
;;
;; The Rust side writes a JSON GerbilCompileRequest to stdin. This executable
;; shim extracts the request source text and recognizes the first smoke form:
;;
;;   (loop-graph gerbil-source-loop
;;     (node provider ask-model (config role planner))
;;     (node tool run-tool (config mode execute))
;;     (edge provider tool success)
;;     (edge tool provider none))
;;
;; It intentionally supports only the unescaped JSON/string subset emitted by
;; the Rust smoke test. A later compiler pass should replace this with a real
;; Gerbil reader and JSON library boundary.
;;
;; The bindings in this file mirror the first Marlin command protocol DTOs on
;; the Gerbil side: a compile response contains one LoopGraph artifact, and a
;; LoopGraph contains node specs plus directed edge specs. Keeping the binding
;; functions explicit prevents the adapter from growing string-spliced JSON
;; directly in the parser.

(define (read-all)
  (let loop ((acc ""))
    (let ((ch (read-char)))
      (if (eof-object? ch)
        acc
        (loop (string-append acc (string ch)))))))

(define (string-at? text start pattern)
  (let ((text-len (string-length text))
        (pattern-len (string-length pattern)))
    (let loop ((index 0))
      (cond
        ((= index pattern-len) #t)
        ((>= (+ start index) text-len) #f)
        ((char=? (string-ref text (+ start index)) (string-ref pattern index))
         (loop (+ index 1)))
        (else #f)))))

(define (find-substring text pattern)
  (let ((text-len (string-length text))
        (pattern-len (string-length pattern)))
    (let loop ((start 0))
      (cond
        ((> (+ start pattern-len) text-len) #f)
        ((string-at? text start pattern) start)
        (else (loop (+ start 1)))))))

(define (extract-json-string-field text field)
  (let* ((pattern (string-append "\"" field "\":\""))
         (start (find-substring text pattern)))
    (if start
      (let ((value-start (+ start (string-length pattern))))
        (let loop ((index value-start))
          (cond
            ((>= index (string-length text)) "")
            ((char=? (string-ref text index) #\") (substring text value-start index))
            (else (loop (+ index 1))))))
      "")))

(define (whitespace? ch)
  (or (char=? ch #\))
      (char=? ch #\space)
      (char=? ch #\newline)
      (char=? ch #\tab)))

(define (delimiter? ch)
  (or (char=? ch #\))
      (char=? ch #\()
      (whitespace? ch)))

(define (tokenize source-text)
  (let ((source-length (string-length source-text)))
    (let loop ((index 0) (tokens '()))
      (cond
        ((>= index source-length) (reverse tokens))
        ((char=? (string-ref source-text index) #\()
         (loop (+ index 1) (cons "(" tokens)))
        ((char=? (string-ref source-text index) #\))
         (loop (+ index 1) (cons ")" tokens)))
        ((whitespace? (string-ref source-text index))
         (loop (+ index 1) tokens))
        (else
         (let scan ((end index))
           (cond
             ((>= end source-length)
              (loop end (cons (substring source-text index end) tokens)))
             ((delimiter? (string-ref source-text end))
              (loop end (cons (substring source-text index end) tokens)))
             (else (scan (+ end 1))))))))))

(define (drop-list tokens)
  (let loop ((remaining tokens) (depth 0))
    (cond
      ((null? remaining) '())
      ((string=? (car remaining) "(")
       (loop (cdr remaining) (+ depth 1)))
      ((string=? (car remaining) ")")
       (if (= depth 1)
         (cdr remaining)
         (loop (cdr remaining) (- depth 1))))
      (else (loop (cdr remaining) depth)))))

(define (extract-loop-graph-id tokens)
  (let loop ((remaining tokens))
    (cond
      ((null? remaining) "gerbil-fixed-loop")
      ((and (string=? (car remaining) "(")
            (pair? (cdr remaining))
            (string=? (cadr remaining) "loop-graph")
            (pair? (cddr remaining)))
       (caddr remaining))
      (else (loop (cdr remaining))))))

;; Gerbil-side bindings for the Marlin LoopGraph protocol shape.
(define (make-marlin-loop-node node-id executor config)
  (list node-id executor config))

(define (marlin-loop-node-id node)
  (car node))

(define (marlin-loop-node-executor node)
  (cadr node))

(define (marlin-loop-node-config node)
  (caddr node))

(define (make-marlin-loop-edge from to condition)
  (list from to condition))

(define (marlin-loop-edge-from edge)
  (car edge))

(define (marlin-loop-edge-to edge)
  (cadr edge))

(define (marlin-loop-edge-condition edge)
  (caddr edge))

(define (make-marlin-loop-graph graph-id nodes edges)
  (list graph-id nodes edges))

(define (marlin-loop-graph-id graph)
  (car graph))

(define (marlin-loop-graph-nodes graph)
  (cadr graph))

(define (marlin-loop-graph-edges graph)
  (caddr graph))

(define (parse-config tokens)
  (if (and (pair? tokens)
           (string=? (car tokens) "(")
           (pair? (cdr tokens))
           (string=? (cadr tokens) "config"))
    (let loop ((remaining (cddr tokens)) (pairs '()))
      (cond
        ((null? remaining) (cons (reverse pairs) '()))
        ((string=? (car remaining) ")")
         (cons (reverse pairs) (cdr remaining)))
        ((pair? (cdr remaining))
         (loop (cddr remaining)
               (cons (list (car remaining) (cadr remaining)) pairs)))
        (else (cons (reverse pairs) remaining))))
    (cons '() tokens)))

(define (parse-node-form tokens)
  (let* ((node-id (caddr tokens))
         (executor (cadddr tokens))
         (after-executor (cddddr tokens))
         (config-result (parse-config after-executor))
         (config (car config-result))
         (after-config (cdr config-result))
         (rest (if (and (pair? after-config)
                        (string=? (car after-config) ")"))
                 (cdr after-config)
                 (drop-list tokens))))
    (cons (make-marlin-loop-node node-id executor config) rest)))

(define (parse-nodes tokens)
  (let loop ((remaining tokens) (nodes '()))
    (cond
      ((null? remaining) (reverse nodes))
      ((and (string=? (car remaining) "(")
            (pair? (cdr remaining))
            (string=? (cadr remaining) "node")
            (pair? (cddr remaining))
            (pair? (cdddr remaining)))
       (let ((node-result (parse-node-form remaining)))
         (loop (cdr node-result) (cons (car node-result) nodes))))
      (else (loop (cdr remaining) nodes)))))

(define (edge-condition-token token)
  (if (or (string=? token "none")
          (string=? token "null")
          (string=? token "#f"))
    #f
    token))

(define (parse-edge-form tokens)
  (let* ((from (caddr tokens))
         (to (cadddr tokens))
         (after-to (cddddr tokens))
         (condition (if (or (null? after-to)
                            (string=? (car after-to) ")"))
                      #f
                      (edge-condition-token (car after-to))))
         (rest (cond
                 ((null? after-to) '())
                 ((string=? (car after-to) ")") (cdr after-to))
                 ((and (pair? (cdr after-to))
                       (string=? (cadr after-to) ")"))
                  (cddr after-to))
                 (else (drop-list tokens)))))
    (cons (make-marlin-loop-edge from to condition) rest)))

(define (parse-edges tokens)
  (let loop ((remaining tokens) (edges '()))
    (cond
      ((null? remaining) (reverse edges))
      ((and (string=? (car remaining) "(")
            (pair? (cdr remaining))
            (string=? (cadr remaining) "edge")
            (pair? (cddr remaining))
            (pair? (cddr remaining))
            (pair? (cdddr remaining)))
       (let ((edge-result (parse-edge-form remaining)))
         (loop (cdr edge-result) (cons (car edge-result) edges))))
      (else (loop (cdr remaining) edges)))))

(define (compile-loop-graph source-text)
  (let* ((tokens (tokenize source-text))
         (graph-id (extract-loop-graph-id tokens))
         (nodes (parse-nodes tokens))
         (edges (parse-edges tokens)))
    (make-marlin-loop-graph graph-id nodes edges)))

(define (display-json-string value)
  (display "\"")
  (let ((value-length (string-length value)))
    (let loop ((index 0))
      (if (< index value-length)
        (begin
          (let ((ch (string-ref value index)))
            (cond
              ((char=? ch #\") (display "\\\""))
              ((char=? ch #\\) (display "\\\\"))
              ((char=? ch #\newline) (display "\\n"))
              ((char=? ch #\tab) (display "\\t"))
              (else (display ch))))
          (loop (+ index 1)))
        #t)))
  (display "\""))

(define (display-json-nullable-string value)
  (if value
    (display-json-string value)
    (display "null")))

(define (display-json-config config)
  (display "{")
  (let loop ((remaining config) (first #t))
    (if (null? remaining)
      #t
      (begin
        (if first #f (display ","))
        (let ((pair (car remaining)))
          (display-json-string (car pair))
          (display ":")
          (display-json-string (cadr pair)))
        (loop (cdr remaining) #f))))
  (display "}"))

(define (display-json-nodes nodes)
  (display "[")
  (let loop ((remaining nodes) (first #t))
    (if (null? remaining)
      #t
      (begin
        (if first #f (display ","))
        (let ((node (car remaining)))
          (display "{\"id\":")
          (display-json-string (marlin-loop-node-id node))
          (display ",\"executor\":")
          (display-json-string (marlin-loop-node-executor node))
          (display ",\"config\":")
          (display-json-config (marlin-loop-node-config node))
          (display "}"))
        (loop (cdr remaining) #f))))
  (display "]"))

(define (display-json-edges edges)
  (display "[")
  (let loop ((remaining edges) (first #t))
    (if (null? remaining)
      #t
      (begin
        (if first #f (display ","))
        (let ((edge (car remaining)))
          (display "{\"from\":")
          (display-json-string (marlin-loop-edge-from edge))
          (display ",\"to\":")
          (display-json-string (marlin-loop-edge-to edge))
          (display ",\"condition\":")
          (display-json-nullable-string (marlin-loop-edge-condition edge))
          (display "}"))
        (loop (cdr remaining) #f))))
  (display "]"))

(define (display-json-loop-graph graph)
  (display "{\"graph_id\":")
  (display-json-string (marlin-loop-graph-id graph))
  (display ",\"nodes\":")
  (display-json-nodes (marlin-loop-graph-nodes graph))
  (display ",\"edges\":")
  (display-json-edges (marlin-loop-graph-edges graph))
  (display "}"))

(define (display-gerbil-compile-response graph)
  (display "{\"artifact\":{\"LoopGraph\":")
  (display-json-loop-graph graph)
  (display "}}"))

(let* ((request (read-all))
       (source-text (extract-json-string-field request "text"))
       (graph (compile-loop-graph source-text)))
  (display-gerbil-compile-response graph)
  (newline))
