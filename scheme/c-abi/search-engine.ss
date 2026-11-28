;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Rooted Scheme values only. Rust (`marlin-agent-search-engine`) decodes the
;;; vector directly into `agent_semantic_search_playbook::SearchPlaybookPlanInput`.
(import (only-in :marlin-agent-core/c-abi/roots marlin-rooted-call)
        (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/search-engine/config SearchEngineScope))
(export marlin-search-engine-plan-project)

(def (marlin-search-engine-plan-project input)
  (if (or (void? input) (not input))
    ((.get SearchEngineScope project))
    ((.get SearchEngineScope project-plan) input)))

(begin-foreign
  (c-define (search-engine-plan-project root) (int64) int64
    "marlin_search_engine_plan_project" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/search-engine#marlin-search-engine-plan-project)))
