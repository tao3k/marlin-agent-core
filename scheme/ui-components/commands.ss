;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Command discovery is a selected UI module. Execution remains with the
;;; Rust command catalog; these entries carry no handler or effect authority.
(import (only-in :clan/poo/object .def .o .get .ref .mix seal-object!))
(export marlin-ui-command-catalog marlin-ui-command-entries
        marlin-ui-command-entries? marlin-ui-browse-command)

(.def UiCommandCatalog.
  entries: '(("/help" "Keyboard shortcuts" #f)
             ("/models" "List configured models" #f)
             ("/model" "Select a model alias" #t)))
(.def SessionCommands.
  (entries => append '(("/sessions" "List sessions; next opens another page" #t)
                       ("/session" "Open a quoted session ID" #t)
                       ("/inspect" "Inspect a quoted session ID" #t)
                       ("/history" "Browse older or newest history" #t))))
(.def WorkspaceCommands.
  (entries => append '(("/status" "Show workspace status" #f)
                       ("/diff" "Show unstaged or staged changes" #t)
                       ("/context" "Show context budget" #f)
                       ("/runtime" "Show runtime policy" #f)
                       ("/clear" "Clear displayed transcript" #f))))
;;; The inherited slot computes one ordered catalog when the selected module
;;; reads it. No additional loader or imperative append registry is needed.
(def marlin-ui-command-catalog
  (seal-object! (.mix WorkspaceCommands. SessionCommands. UiCommandCatalog.)))

(def (command-entry? entry)
  (and (pair? entry) (pair? (cdr entry)) (pair? (cddr entry))
       (null? (cdddr entry))
       (string? (car entry)) (<= 2 (string-length (car entry)) 32)
       (string-prefix? "/" (car entry))
       (string? (cadr entry)) (<= (string-length (cadr entry)) 256)
       (boolean? (caddr entry))))

;;; Admission is bounded before map/filter can traverse a supplied catalog.
(def (marlin-ui-command-entries? entries)
  (let loop ((rest entries) (remaining 32))
    (cond ((null? rest) #t)
          ((or (zero? remaining) (not (pair? rest))
               (not (command-entry? (car rest)))) #f)
          (else (loop (cdr rest) (- remaining 1))))))

(def (marlin-ui-command-entries catalog)
  (let (entries (.get catalog entries))
    (unless (marlin-ui-command-entries? entries) (error "invalid UI command catalog"))
    (map (lambda (entry)
           (list (string-copy (car entry)) (string-copy (cadr entry)) (caddr entry)))
         entries)))

;;; A browse result is a presentation proposal, checked again by the host.
(def (marlin-ui-browse-command entries event)
  (let* ((query (.ref event 'query))
         (previous (.ref event 'previous))
         (direction (.ref event 'direction)))
    (unless (and (marlin-ui-command-entries? entries)
                 (string? query) (<= 1 (string-length query) 32)
                 (or (not previous) (and (string? previous) (<= (string-length previous) 32)))
                 (memq direction '(next previous)))
      (error "invalid command browse event or catalog"))
    (let* ((matches (map car (filter (lambda (entry) (string-prefix? query (car entry))) entries)))
           (found (and previous (member previous matches)))
           (choice (cond ((null? matches) #f)
                         ((not found) (if (eq? direction 'next) (car matches) (car (reverse matches))))
                         ((eq? direction 'next) (if (pair? (cdr found)) (cadr found) (car matches)))
                         (else (let loop ((rest matches) (prior (car (reverse matches))))
                                 (if (equal? (car rest) previous) prior
                                     (loop (cdr rest) (car rest))))))))
      (.o kind: 'command-selection name: choice))))
