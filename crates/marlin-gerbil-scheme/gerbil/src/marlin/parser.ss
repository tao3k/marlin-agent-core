;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; Minimal Marlin loop-graph source parser used by Gerbil-side adapters.

package: marlin

(import ./protocol)

(export compile-loop-graph
        compile-workspace-schema
        compile-workspace-patch-intent
        compile-agent-scenario-contract
        compile-release-topology)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (source->form source-text)
  (let ((form (read (open-input-string source-text))))
    (if (eof-object? form)
      (error "empty loop graph source")
      form)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (atom->string value)
  (cond
    ((string? value) value)
    ((symbol? value) (symbol->string value))
    (else (error "expected symbol or string" value))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (form-tag? form tag)
  (and (pair? form)
       (symbol? (car form))
       (eq? (car form) tag)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-config-entries entries)
  (cond
   ((null? entries) '())
   ((pair? (cdr entries))
    (cons (list (atom->string (car entries))
                (atom->string (cadr entries)))
          (parse-config-entries (cddr entries))))
   (else (error "config form expects key/value pairs" entries))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-config-form form)
  (if (form-tag? form 'config)
    (parse-config-entries (cdr form))
    '()))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-node-config forms)
  (apply append
         (map parse-config-form
              (filter (cut form-tag? <> 'config) forms))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-node-form form)
  (if (and (form-tag? form 'node)
           (pair? (cdr form))
           (pair? (cddr form)))
    (make-marlin-loop-node (atom->string (cadr form))
                           (atom->string (caddr form))
                           (parse-node-config (cdddr form)))
    (error "invalid node form" form)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (edge-condition-value value)
  (cond
    ((not value) #f)
    ((and (symbol? value)
          (or (eq? value 'none)
              (eq? value 'null)))
     #f)
    (else (atom->string value))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
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

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-nodes forms)
  (map parse-node-form
       (filter (cut form-tag? <> 'node) forms)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-edges forms)
  (map parse-edge-form
       (filter (cut form-tag? <> 'edge) forms)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
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

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (atom-list->strings values)
  (map atom->string values))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (tagged-values forms tag)
  (apply append
         (map (lambda (form)
                (atom-list->strings (cdr form)))
              (filter (cut form-tag? <> tag) forms))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
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

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (find-tagged-form forms tag)
  (find (cut form-tag? <> tag) forms))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (required-single-value forms tag)
  (let ((form (find-tagged-form forms tag)))
    (if (and form (pair? (cdr form)))
      (atom->string (cadr form))
      (error "expected single-value form" tag forms))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (optional-single-value forms tag)
  (let ((form (find-tagged-form forms tag)))
    (if form
      (if (pair? (cdr form))
        (atom->string (cadr form))
        (error "expected single-value form" tag forms))
      #f)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (true-value? value)
  (cond
    ((eq? value #t) #t)
    ((and (symbol? value)
          (or (eq? value 'true)
              (eq? value 'yes)))
     #t)
    (else #f)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (required-boolean-value forms tag)
  (let ((form (find-tagged-form forms tag)))
    (if (and form (pair? (cdr form)))
      (true-value? (cadr form))
      (error "expected boolean form" tag forms))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (required-dry-run-first forms)
  (let ((form (find-tagged-form forms 'dry-run-first)))
    (if (and form (pair? (cdr form)) (true-value? (cadr form)))
      #t
      (error "workspace-patch-intent requires dry-run-first true" forms))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-set-todo-op form)
  (if (and (form-tag? form 'set-todo)
           (pair? (cdr form))
           (pair? (cddr form)))
    (make-marlin-set-todo-op (atom->string (cadr form))
                             (atom->string (caddr form)))
    (error "invalid set-todo form" form)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-set-property-op form)
  (if (and (form-tag? form 'set-property)
           (pair? (cdr form))
           (pair? (cddr form))
           (pair? (cdddr form)))
    (make-marlin-set-property-op (atom->string (cadr form))
                                 (atom->string (caddr form))
                                 (atom->string (cadddr form)))
    (error "invalid set-property form" form)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-mark-memory-candidate-op form)
  (if (and (form-tag? form 'mark-memory-candidate)
           (pair? (cdr form))
           (pair? (cddr form)))
    (make-marlin-mark-memory-candidate-op (atom->string (cadr form))
                                          (atom->string (caddr form)))
    (error "invalid mark-memory-candidate form" form)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-workspace-patch-op form)
  (cond
    ((form-tag? form 'set-todo)
     (parse-set-todo-op form))
    ((form-tag? form 'set-property)
     (parse-set-property-op form))
    ((form-tag? form 'mark-memory-candidate)
     (parse-mark-memory-candidate-op form))
    (else (error "unsupported workspace patch op form" form))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-workspace-patch-ops forms)
  (map parse-workspace-patch-op
       (filter
        (lambda (form)
          (not (or (form-tag? form 'reason)
                   (form-tag? form 'source-agent))))
        forms)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-workspace-patch-form form)
  (if (form-tag? form 'patch)
    (let ((forms (cdr form)))
      (make-marlin-workspace-patch
       (required-single-value forms 'reason)
       (optional-single-value forms 'source-agent)
       (parse-workspace-patch-ops forms)))
    (error "expected patch form" form)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
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

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-scenario-input form)
  (if (and (form-tag? form 'input)
           (pair? (cdr form))
           (pair? (cddr form)))
    (list (atom->string (cadr form))
          (atom->string (caddr form)))
    (error "invalid scenario input form" form)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-scenario-step-inputs forms)
  (map parse-scenario-input
       (filter (cut form-tag? <> 'input) forms)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
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

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-scenario-steps forms)
  (map parse-scenario-step-form
       (filter (cut form-tag? <> 'step) forms)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
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

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-release-visibility-form form)
  (if (form-tag? form 'visibility)
    (let ((forms (cdr form)))
      (make-marlin-release-visibility
       (required-single-value forms 'report-key)
       (tagged-values forms 'evidence-keys)
       (tagged-values forms 'artifact-paths)))
    (error "invalid release visibility form" form)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-release-visibility forms)
  (map parse-release-visibility-form
       (filter (cut form-tag? <> 'visibility) forms)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-release-gate-form form)
  (if (and (form-tag? form 'gate)
           (pair? (cdr form)))
    (let ((gate-id (atom->string (cadr form)))
          (forms (cddr form)))
      (make-marlin-release-gate
       gate-id
       (required-single-value forms 'command)
       (required-boolean-value forms 'requires-local-gerbil)
       (tagged-values forms 'required-artifacts)
       (parse-release-visibility forms)))
    (error "invalid release gate form" form)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (parse-release-gates forms)
  (map parse-release-gate-form
       (filter (cut form-tag? <> 'gate) forms)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (compile-release-topology source-text)
  (let ((form (source->form source-text)))
    (if (and (form-tag? form 'release-topology)
             (pair? (cdr form)))
      (let ((topology-id (atom->string (cadr form)))
            (forms (cddr form)))
        (make-marlin-release-topology
         topology-id
         (required-single-value forms 'crate)
         (required-boolean-value forms 'publish-enabled)
         (required-single-value forms 'asset-audit-command)
         (tagged-values forms 'package-assets)
         (tagged-values forms 'runtime-dependency-chain)
         (tagged-values forms 'workflow-dependency-chain)
         (parse-release-gates forms)))
      (error "expected release-topology form" form))))
