;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :marlin-agent-core/c-abi/roots marlin-rooted-call)
        (only-in :clan/poo/object .get .ref .slot?)
        (only-in :marlin-agent-core/artifacts/objects DefaultArtifactCatalog)
        (only-in :tao3k/yuanxu/prelude yuanxu-build-autoloads)
        (for-syntax (only-in "./build.ss" Build)))
(export dispatch-exchange)

(yuanxu-build-autoloads Build artifacts)

;;; Pure POO workspace-layout projection does not force Lab/SQLite on TUI startup
;;; and requires zero runtime `GERBIL_LOADPATH`.
(def (project-default-workspace-layout)
  (let* ((catalog DefaultArtifactCatalog)
         (org (.get catalog organization))
         (layout (.get catalog workspace-layout))
         (session-db (.get layout session-db))
         (context-db (.get layout context-db))
         (search-dir (.get layout search-dir))
         (ledger-suffix (.get layout process-ledger-suffix))
         (project-id (string-append (.get layout project-prefix) (.get catalog project)))
         (default-session-id (string-append (.get layout session-prefix) (.get layout default-session)))
         (agent-key (.get layout default-agent-key))
         (agents (.get org agents)))
    (unless (and (symbol? agent-key) (.slot? agents agent-key))
      (error "Workspace layout default-agent-key is undeclared in Organization" agent-key))
    (vector session-db
            context-db
            search-dir
            ledger-suffix
            project-id
            default-session-id
            (.get (.ref agents agent-key) identity))))

(def (dispatch-exchange request)
  (if (and (vector? request)
           (= (vector-length request) 1)
           (eq? (vector-ref request 0) 'workspace-layout))
    (project-default-workspace-layout)
    (exchange request)))

(begin-foreign
  (c-define (artifacts-exchange root) (int64) int64
    "marlin_context_artifacts_exchange" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/artifacts#dispatch-exchange)))
