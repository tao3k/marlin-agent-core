;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Exercise Marlin's publications through the selected package entry.

(import (only-in :clan/poo/object .get .o)
        (only-in :agentic-ai-lab/modules/ai-agentic-context/objects AiAgenticContextScope.)
        (only-in :clan/poo/mop TypeError?)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports))

(let (admit (.get marlin-agent-core.init.exports Marlin context admit-scope))
  (def (admit-turn session-id turn-index)
    (admit (.o (:: @ AiAgenticContextScope.) bundle: "bundle" organization: "org" epoch: 1
               project: "project" worktree: "worktree" source-cut: "cut:0"
               actor: "actor" task: "task" destination: "provider"
               session: session-id turn: turn-index)))
  (let (scope (admit-turn "session" 0))
    (unless (and (eq? (.get scope kind) 'agentic-ai-lab.ai-agentic-context.scope.v1)
                 (equal? (.get scope session) "session")
                 (= (.get scope turn) 0)
                 (string? (.get scope semantic-digest)))
      (error "Marlin did not receive a Lab-admitted Context Scope" scope)))
  (unless (with-catch TypeError? (lambda () (admit-turn "session" -1) #f))
    (error "Marlin's Context request contract must refuse a negative turn"))
  (unless (with-catch TypeError? (lambda () (admit-turn #f 0) #f))
    (error "Lab's Scope type must refuse a Context turn without a session")))

(let* ((select (.get marlin-agent-core.init.exports Marlin hooks select))
       (selection (select #f "init-test" (.o) (.o) "test" "local")))
  (unless (and (equal? (.get selection source) "init-test")
               (not (.get selection matched))
               (not (.get selection dynamic-hook-action)))
    (error "Hooks must publish its decision selector through init" selection)))
