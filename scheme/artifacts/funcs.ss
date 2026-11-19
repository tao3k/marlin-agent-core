;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .get .ref .slot? .o)
        (only-in :std/list/list filter-map)
        (only-in :marlin-agent-core/context-system/org-source marlin-context-org-source-graph)
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/runtime/durable-journal
                 journal-open journal-close! journal-receive! journal-begin! journal-commit!
                 journal-put! journal-get journal-rows journal-verify! journal-pack journal-hash)
        (only-in "./objects.ss" ArtifactKinds))
(export artifacts-exchange)

(def (hex-id? value lengths)
  (and (string? value) (memv (string-length value) lengths)
       (andmap (lambda (char)
                 (or (char<=? #\0 char #\9) (char<=? #\a char #\f)))
               (string->list value))))

(def (artifact-record kind-value commit-value worktree-value digest-value)
  (unless (and (symbol? kind-value) (.slot? ArtifactKinds kind-value)
               (hex-id? commit-value '(40 64)) (string? worktree-value)
               (hex-id? digest-value '(64)))
    (error "Invalid artifact evidence"))
  (.o kind: 'marlin.artifact
      artifact-kind: (.get (.ref ArtifactKinds kind-value) kind)
      commit: commit-value worktree: worktree-value digest: digest-value))

(def (record! journal record)
  (def id (journal-hash (journal-pack record)))
  (unless (journal-get journal id)
    (journal-receive! journal id record)
    (journal-begin! journal id)
    (journal-put! journal id record)
    (journal-commit! journal)))

(def (render-context records)
  (call-with-output-string
    (lambda (port)
      (parameterize ((current-output-port port))
        (displayln "#+title: Shared Git Context")
        (for-each
          (lambda (row)
            (def record (.get row payload))
            (displayln "* " (.get record artifact-kind))
            (displayln ":PROPERTIES:")
            (displayln ":ID: " (.get row id))
            (displayln ":GIT_COMMIT: " (.get record commit))
            (displayln ":ARTIFACT: " (.get record digest))
            (displayln ":END:")
            (displayln "[[file:artifacts/" (.get record digest) "][Artifact]]"))
          records)))))

(def (recover-context journal)
  (def records
    (filter-map
      (lambda (row)
        (and (equal? (.get row status) "applied")
             (eq? (.get row payload kind) 'marlin.artifact)
             row))
      (journal-rows journal)))
  (def source (render-context records))
  (def graph (.get (marlin-context-org-source-graph source) graph))
  (def field-of (.get graph field-of))
  (def references
    (filter-map
      (lambda (element)
        (and (equal? (.get element kind) "headline")
             (let* ((id (field-of element "property:ID"))
                    (record (and id (journal-get journal id)))
                    (digest (field-of element "property:ARTIFACT")))
               (unless (and record
                            (equal? digest (.get record digest))
                            (equal? (field-of element "property:GIT_COMMIT") (.get record commit)))
                 (error "Org reference differs from the committed artifact"))
               digest)))
      (.get graph records)))
  (unless (= (length references) (length records))
    (error "Missing Org artifact records"))
  (vector source (list->vector references)))

(def (artifacts-exchange request)
  (def (with-journal path action)
    (def journal (journal-open path))
    (dynamic-wind void
      (lambda () (journal-verify! journal) (action journal))
      (lambda () (journal-close! journal))))
  (match request
    (#('restore path)
     (with-journal path recover-context))
    (#('record path kind commit worktree digest)
     (def record (artifact-record kind commit worktree digest))
     (with-journal path
       (lambda (journal)
         (record! journal record)
         (recover-context journal))))
    (else (error "Invalid artifact request"))))
