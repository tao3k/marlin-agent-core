;;; Batch output must match Lab's individual admissions in request order.

(import (only-in :clan/poo/object .get .o)
        (only-in :agentic-ai-lab/modules/ai-agentic-context/objects AiAgenticContextScope.)
        (only-in :std/encoding/hex hex-decode)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports)
        "../context/batch.ss")

(def shared #("bundle" "org" "project" "worktree" "cut:0"))
(def batch
  (vector shared (u64vector 1 0 18446744073709551615)
          #(#("reader" "first" "provider")
            #("editor" "second" "provider" "session"))))
(def admit (.get marlin-agent-core.init.exports Marlin context admit-scope))
(def (expected actor-id task-id session-id turn-index)
  (def scope
    (admit (.o (:: @ AiAgenticContextScope.) bundle: "bundle" organization: "org" epoch: 1
               project: "project" worktree: "worktree" source-cut: "cut:0"
               actor: actor-id task: task-id destination: "provider"
               session: session-id turn: turn-index)))
  (hex-decode (.get scope semantic-digest) 7))

(let (digests (marlin-context-admit-batch batch))
  (unless (and (= (u8vector-length digests) 64)
               (equal? (subu8vector digests 0 32) (expected "reader" "first" #f #f))
               (equal? (subu8vector digests 32 64)
                       (expected "editor" "second" "session" 18446744073709551615)))
    (error "Context batch did not preserve Lab's digests and request order")))

(unless (with-catch (lambda (_) #t)
          (lambda ()
            (marlin-context-admit-batch
             (vector shared (u64vector 1 0 1)
                     #(#("reader" "first" "provider")
                       #("editor" "second" "provider" #f))))
            #f))
  (error "a malformed later row must fail the whole Context batch"))
