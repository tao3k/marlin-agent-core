;;; -*- Gerbil -*-
;;; Minimal Marlin loop-graph source parser used by Gerbil-side adapters.

package: marlin

(import ./protocol)

(export compile-loop-graph
        compile-workspace-schema
        compile-workspace-patch-intent
        compile-agent-scenario-contract)

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

(define (find-tagged-form forms tag)
  (let loop ((remaining forms))
    (cond
      ((null? remaining) #f)
      ((form-tag? (car remaining) tag) (car remaining))
      (else (loop (cdr remaining))))))

(define (required-single-value forms tag)
  (let ((form (find-tagged-form forms tag)))
    (if (and form (pair? (cdr form)))
      (atom->string (cadr form))
      (error "expected single-value form" tag forms))))

(define (optional-single-value forms tag)
  (let ((form (find-tagged-form forms tag)))
    (if form
      (if (pair? (cdr form))
        (atom->string (cadr form))
        (error "expected single-value form" tag forms))
      #f)))

(define (true-value? value)
  (cond
    ((eq? value #t) #t)
    ((and (symbol? value)
          (or (eq? value 'true)
              (eq? value 'yes)))
     #t)
    (else #f)))

(define (required-dry-run-first forms)
  (let ((form (find-tagged-form forms 'dry-run-first)))
    (if (and form (pair? (cdr form)) (true-value? (cadr form)))
      #t
      (error "workspace-patch-intent requires dry-run-first true" forms))))

(define (parse-mark-memory-candidate-op form)
  (if (and (form-tag? form 'mark-memory-candidate)
           (pair? (cdr form))
           (pair? (cddr form)))
    (make-marlin-mark-memory-candidate-op (atom->string (cadr form))
                                          (atom->string (caddr form)))
    (error "invalid mark-memory-candidate form" form)))

(define (parse-workspace-patch-op form)
  (cond
    ((form-tag? form 'mark-memory-candidate)
     (parse-mark-memory-candidate-op form))
    (else (error "unsupported workspace patch op form" form))))

(define (parse-workspace-patch-ops forms)
  (let loop ((remaining forms) (ops '()))
    (cond
      ((null? remaining) (reverse ops))
      ((or (form-tag? (car remaining) 'reason)
           (form-tag? (car remaining) 'source-agent))
       (loop (cdr remaining) ops))
      (else
       (loop (cdr remaining)
             (cons (parse-workspace-patch-op (car remaining)) ops))))))

(define (parse-workspace-patch-form form)
  (if (form-tag? form 'patch)
    (let ((forms (cdr form)))
      (make-marlin-workspace-patch
       (required-single-value forms 'reason)
       (optional-single-value forms 'source-agent)
       (parse-workspace-patch-ops forms)))
    (error "expected patch form" form)))

(define (compile-workspace-patch-intent source-text)
  (let ((form (source->form source-text)))
    (if (and (form-tag? form 'workspace-patch-intent)
             (pair? (cdr form)))
      (let ((intent-id (atom->string (cadr form)))
            (forms (cddr form)))
        (make-marlin-workspace-patch-intent
         intent-id
         (parse-workspace-patch-form (find-tagged-form forms 'patch))
         (required-dry-run-first forms)))
      (error "expected workspace-patch-intent form" form))))

(define (parse-scenario-input form)
  (if (and (form-tag? form 'input)
           (pair? (cdr form))
           (pair? (cddr form)))
    (list (atom->string (cadr form))
          (atom->string (caddr form)))
    (error "invalid scenario input form" form)))

(define (parse-scenario-step-inputs forms)
  (let loop ((remaining forms) (inputs '()))
    (cond
      ((null? remaining) (reverse inputs))
      ((form-tag? (car remaining) 'input)
       (loop (cdr remaining)
             (cons (parse-scenario-input (car remaining)) inputs)))
      (else (loop (cdr remaining) inputs)))))

(define (parse-scenario-step-form form)
  (if (and (form-tag? form 'step)
           (pair? (cdr form)))
    (let ((name (atom->string (cadr form)))
          (forms (cddr form)))
      (make-marlin-agent-scenario-step
       name
       (parse-scenario-step-inputs forms)
       (tagged-values forms 'event-topic)
       (tagged-values forms 'span-name)))
    (error "invalid agent scenario step form" form)))

(define (parse-scenario-steps forms)
  (let loop ((remaining forms) (steps '()))
    (cond
      ((null? remaining) (reverse steps))
      ((form-tag? (car remaining) 'step)
       (loop (cdr remaining)
             (cons (parse-scenario-step-form (car remaining)) steps)))
      (else (loop (cdr remaining) steps)))))

(define (compile-agent-scenario-contract source-text)
  (let ((form (source->form source-text)))
    (if (and (form-tag? form 'agent-scenario-contract)
             (pair? (cdr form)))
      (let ((scenario-id (atom->string (cadr form)))
            (forms (cddr form)))
        (make-marlin-agent-scenario-contract
         (make-marlin-agent-scenario
          scenario-id
          (optional-single-value forms 'description)
          (parse-scenario-steps forms)
          (tagged-values forms 'evidence))))
      (error "expected agent-scenario-contract form" form))))
