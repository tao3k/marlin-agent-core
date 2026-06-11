;;; -*- Gerbil -*-
;;; Minimal Marlin loop-graph source parser used by Gerbil-side adapters.

package: marlin

(import ./protocol)

(export compile-loop-graph
        compile-workspace-schema)

(define (source->form source-text)
  (let ((form (read (open-input-string source-text))))
    (if (eof-object? form)
      (error "empty loop graph source")
      form)))

(define (atom->string value)
  (cond
    ((string? value) value)
    ((symbol? value) (symbol->string value))
    (else (error "expected symbol or string" value))))

(define (form-tag? form tag)
  (and (pair? form)
       (symbol? (car form))
       (eq? (car form) tag)))

(define (parse-config-entries entries)
  (let loop ((remaining entries) (pairs '()))
    (cond
      ((null? remaining) (reverse pairs))
      ((pair? (cdr remaining))
       (loop (cddr remaining)
             (cons (list (atom->string (car remaining))
                         (atom->string (cadr remaining)))
                   pairs)))
      (else (error "config form expects key/value pairs" entries)))))

(define (parse-config-form form)
  (if (form-tag? form 'config)
    (parse-config-entries (cdr form))
    '()))

(define (parse-node-config forms)
  (let loop ((remaining forms) (config '()))
    (cond
      ((null? remaining) (reverse config))
      ((form-tag? (car remaining) 'config)
       (loop (cdr remaining) (append (reverse (parse-config-form (car remaining))) config)))
      (else (loop (cdr remaining) config)))))

(define (parse-node-form form)
  (if (and (form-tag? form 'node)
           (pair? (cdr form))
           (pair? (cddr form)))
    (make-marlin-loop-node (atom->string (cadr form))
                           (atom->string (caddr form))
                           (parse-node-config (cdddr form)))
    (error "invalid node form" form)))

(define (edge-condition-value value)
  (cond
    ((not value) #f)
    ((and (symbol? value)
          (or (eq? value 'none)
              (eq? value 'null)))
     #f)
    (else (atom->string value))))

(define (parse-edge-form form)
  (if (and (form-tag? form 'edge)
           (pair? (cdr form))
           (pair? (cddr form)))
    (make-marlin-loop-edge (atom->string (cadr form))
                           (atom->string (caddr form))
                           (if (pair? (cdddr form))
                             (edge-condition-value (cadddr form))
                             #f))
    (error "invalid edge form" form)))

(define (parse-nodes forms)
  (let loop ((remaining forms) (nodes '()))
    (cond
      ((null? remaining) (reverse nodes))
      ((form-tag? (car remaining) 'node)
       (loop (cdr remaining) (cons (parse-node-form (car remaining)) nodes)))
      (else (loop (cdr remaining) nodes)))))

(define (parse-edges forms)
  (let loop ((remaining forms) (edges '()))
    (cond
      ((null? remaining) (reverse edges))
      ((form-tag? (car remaining) 'edge)
       (loop (cdr remaining) (cons (parse-edge-form (car remaining)) edges)))
      (else (loop (cdr remaining) edges)))))

(define (compile-loop-graph source-text)
  (let ((form (source->form source-text)))
    (if (and (form-tag? form 'loop-graph)
             (pair? (cdr form)))
      (let ((graph-id (atom->string (cadr form)))
            (forms (cddr form)))
        (make-marlin-loop-graph graph-id
                                (parse-nodes forms)
                                (parse-edges forms)))
      (error "expected loop-graph form" form))))

(define (atom-list->strings values)
  (let loop ((remaining values) (strings '()))
    (if (null? remaining)
      (reverse strings)
      (loop (cdr remaining) (cons (atom->string (car remaining)) strings)))))

(define (tagged-values forms tag)
  (let loop ((remaining forms) (values '()))
    (cond
      ((null? remaining) (reverse values))
      ((form-tag? (car remaining) tag)
       (loop (cdr remaining)
             (append (reverse (atom-list->strings (cdar remaining))) values)))
      (else (loop (cdr remaining) values)))))

(define (compile-workspace-schema source-text)
  (let ((form (source->form source-text)))
    (if (and (form-tag? form 'workspace-schema)
             (pair? (cdr form)))
      (let ((schema-id (atom->string (cadr form)))
            (forms (cddr form)))
        (make-marlin-workspace-schema schema-id
                                      (tagged-values forms 'required)
                                      (tagged-values forms 'todo)))
      (error "expected workspace-schema form" form))))
