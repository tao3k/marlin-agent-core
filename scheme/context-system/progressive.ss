;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Production Context System facade over Lab's Org Elements -> ISO GQL,
;;; ASCENT progressive disclosure (`L0..L3` + drill-down + dynamic Flash facets),
;;; two-stage AST/Flash Org repair, and parser-verified Org structural updates.
(import (only-in :clan/poo/object .get .o .ref .slot? object?)
        (only-in :std/list/list filter-map)
        (only-in :marlin-agent-core/interface LabContext))
(export marlin-context-element-graph
        marlin-context-select-elements
        marlin-context-resolve-attachment-path
        marlin-context-progressive-dispatch
        marlin-context-repair-org
        marlin-context-babel-upsert
        marlin-context-checklist-set
        marlin-context-headline-set-todo
        marlin-context-fission-subtree)

(def (opt-slot obj key default)
  (if (and (object? obj) (.slot? obj key))
    (.ref obj key)
    default))

(def (marlin-context-element-graph request)
  (unless (object? request)
    (error "Element graph request must be a POO object"))
  ((.get LabContext org-element-graph)
   (opt-slot request 'identity 'marlin/org-elements)
   (.ref request 'sources)
   (opt-slot request 'complete? #t)
   (opt-slot request 'tz-offset 0)))

(def (marlin-context-select-elements request)
  (unless (object? request)
    (error "Element selection request must be a POO object"))
  ((.get LabContext org-elements-select)
   (.ref request 'element-graph)
   (.ref request 'selector)
   (opt-slot request 'query-id 'marlin/org-elements-gql)
   (opt-slot request 'bound 128)))

(def (marlin-context-resolve-attachment-path owner-root attach-dir target-path)
  (unless (and (string? target-path) (not (string-empty? target-path)))
    (error "Org attachment link requires a non-empty target path"))
  (let ((rel (if (and (string? attach-dir) (not (string-empty? attach-dir)))
               (string-append attach-dir "/" target-path)
               (string-append "data/" target-path))))
    (if (and (string? owner-root) (not (string-empty? owner-root)))
      (path-expand rel owner-root)
      rel)))

(def (marlin-context-progressive-dispatch request)
  (unless (object? request)
    (error "Progressive dispatch request must be a POO object"))
  (let* ((base-cfg (opt-slot request 'rule-config
                             (.get LabContext progressive-rule-config-prototype)))
         (context-org (opt-slot request 'context-org #f))
         (rule-config (if (string? context-org)
                        ((.get LabContext parse-rule-overrides) context-org base-cfg)
                        base-cfg))
         (element-graph ((.get LabContext org-element-graph)
                         (opt-slot request 'graph-identity 'marlin/progressive-context)
                         (.ref request 'sources)
                         (opt-slot request 'complete? #t)
                         (opt-slot request 'tz-offset 0)))
         (ready-tasks ((.get LabContext ready-task-slices)
                       element-graph
                       (opt-slot request 'tier-filter #f)))
         (focus-property-ids (opt-slot request 'focus-property-ids '()))
         (defeater-property-ids (opt-slot request 'defeater-property-ids '()))
         (drill-downs (opt-slot request 'drill-downs '()))
         (case-signals (opt-slot request 'case-signals '()))
         (progressive-plan
          ((.get LabContext ascent-progressive-plan)
           element-graph
           (.o rule-config focus-property-ids defeater-property-ids
               drill-downs case-signals)))
         (progressive-org
          ((.get LabContext render-progressive-org)
           element-graph progressive-plan
           (opt-slot request 'source-filter #f)))
         (attach-sel
          ((.get LabContext org-elements-select)
           element-graph
           '(org-elements link (child-of headline) (property protocol "attachment"))
           'marlin/org-attach-links 128))
         (owner-root (opt-slot request 'owner-root #f))
         (resolved-attachments
          (filter-map
           (lambda (ln)
             (let* ((source-id (.ref ln 'sourceId))
                    (headline-id (.ref ln 'headlinePropertyId))
                    (target-path (.ref ln 'targetPath))
                    (attach-dir (.ref ln 'attachDir))
                    (image? (.ref ln 'imageLink))
                    (resolved-path
                     (marlin-context-resolve-attachment-path owner-root attach-dir target-path)))
               (.o source-id headline-id attach-dir target-path image? resolved-path)))
           (.ref attach-sel 'nodes)))
         (kind 'marlin.context.progressive-dispatch.v1)
         (action-authorized? #f))
    (.o kind rule-config element-graph ready-tasks
        progressive-plan progressive-org resolved-attachments
        action-authorized?)))

(def (marlin-context-repair-org request)
  (unless (object? request)
    (error "Org repair request must be a POO object"))
  (let ((defaults (opt-slot request 'defaults #f))
        (catalog (opt-slot request 'catalog #f)))
    (if catalog
      ((.get LabContext repair-org)
       (.ref request 'source-id)
       (.ref request 'generation)
       (.ref request 'source)
       defaults
       catalog)
      ((.get LabContext repair-org)
       (.ref request 'source-id)
       (.ref request 'generation)
       (.ref request 'source)
       defaults))))

(def (marlin-context-babel-upsert request)
  ((.get LabContext babel-upsert-results)
   (.ref request 'source)
   (.ref request 'block-name)
   (.ref request 'body)
   (opt-slot request 'thinking #f)))

(def (marlin-context-checklist-set request)
  ((.get LabContext checklist-set)
   (.ref request 'source)
   (.ref request 'headline-property-id)
   (.ref request 'item-index)
   (.ref request 'checked?)))

(def (marlin-context-headline-set-todo request)
  ((.get LabContext headline-set-todo)
   (.ref request 'source)
   (.ref request 'headline-property-id)
   (.ref request 'keyword)))

(def (marlin-context-fission-subtree request)
  ((.get LabContext fission-subtree)
   (.ref request 'scratch-cut)
   (.ref request 'target-org)
   (.ref request 'target-property-id)
   (opt-slot request 'origin-module "context.org-babel-fission")))
