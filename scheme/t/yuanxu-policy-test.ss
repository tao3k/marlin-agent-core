;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Exercise Marlin's publications through the selected package entry.

(import (only-in :marlin-agent-core/interface LabContext)
        (only-in :marlin-agent-core/scheme-package-hooks/funcs run-ui-package-hooks)
        (only-in :tao3k/yuanxu/module-system/module-hooks ModuleHook. make-module-hooks)
        (only-in :clan/poo/object .call .get .o)
        (only-in :clan/poo/type TypeError?)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports))

(let (admit (.get marlin-agent-core.init.exports Marlin context-system admit-scope))
  (def (admit-turn session-id turn-index)
    (admit (.o (:: @ (.get LabContext scope-prototype)) bundle: "bundle" organization: "org" epoch: 1
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

(let* ((select (.get marlin-agent-core.init.exports Marlin agent-hooks select))
       (selection (select #f "init-test" (.o) (.o) "test" "local")))
  (unless (and (equal? (.get selection source) "init-test")
               (not (.get selection matched))
               (not (.get selection dynamic-hook-action)))
    (error "Agent Hooks must publish its decision selector through init" selection)))

(let (package-hooks (.get marlin-agent-core.init.exports Marlin scheme-package-hooks))
  (unless (eq? (.call package-hooks .run 'unrelated-scene (.o) 'untouched) 'untouched)
    (error "an unrelated scene must leave the Scheme package result alone")))

(let* ((hooks
         (make-module-hooks
           (.o fixture: (.o (:: @ ModuleHook.) on: '(ui-command-catalog)
                            .run: (lambda (_request _result) 'publication))
               customize: (.o (:: @ ModuleHook.) on: '(ui-package-loaded)
                              .run: (lambda (request publication)
                                      (list (.get request trigger) publication))))))
       (result (run-ui-package-hooks hooks 'ui-command-catalog
                 (.o trigger: 'ui-command-catalog))))
  (unless (equal? result '(ui-command-catalog publication))
    (error "post-load hooks did not receive the publication and its demand" result)))
