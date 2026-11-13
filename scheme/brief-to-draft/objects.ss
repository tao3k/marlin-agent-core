;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .o .ref)
        (only-in :std/list/list find)
        (only-in :orgize/languages/org/modules/org-elements/values
                 org-element-graph-records org-element-graph-kind-of org-element-graph-field-of)
        (only-in :marlin-agent-core/context/org-source marlin-context-org-source-graph)
        (only-in "./settings.ss" settings-denial))

(export BriefToDraftOperations)

(def (project-settings input)
  (let* ((bundle-audience (vector-ref input 0))
         (bundle-template (vector-ref input 1))
         (project-audience (vector-ref input 2))
         (project-template (vector-ref input 3))
         (audience (cond ((boolean? project-audience) project-audience)
                         ((boolean? bundle-audience) bundle-audience)
                         (else #t)))
         (template (or project-template bundle-template 'brief-note))
         (denial (settings-denial audience template)))
    (if (null? denial)
      (vector #t audience (if (eq? template 'brief-note) 0 1))
      (vector #f #f (if (null? (cdr denial)) 1 2)))))

(def (project-proposal input)
  (let* ((source (vector-ref input 0))
         (required? (vector-ref input 1))
         (template (vector-ref input 2))
         (graph (.ref (marlin-context-org-source-graph source) 'graph))
         (kind (org-element-graph-kind-of graph))
         (field (org-element-graph-field-of graph))
         (headline (find (lambda (element) (equal? (kind element) "headline"))
                         (org-element-graph-records graph)))
         (title (and headline (field headline "title")))
         (audience (and headline (field headline "property:AUDIENCE"))))
    (and (string? title) (positive? (string-length title))
         (or (not required?) (and (string? audience) (positive? (string-length audience))))
         (vector title (or audience "")
                 (vector (if (eq? template 'release-note) 1 0) 2)))))

(def BriefToDraftOperations
  (.o identity: 'marlin/brief-to-draft validate-settings: project-settings propose: project-proposal))
