;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; -*- Gerbil -*-
;;; Integration tests for GraphEngine × ai-agentic-context Context Governance:
;;;   1. Live `make-governed-context-trunk` + Two-Zone KV-Prefix digest +
;;;      Design 165 `blocker-active` runtime overlay (`auto-archive: #f`);
;;;   2. Active defeater forces L3 disclosure floor (rejects L2 fail-closed);
;;;   3. Slice 52 Lethal-Trifecta Nono sandbox denial & Cedar handoff validation;
;;;   4. 13-slot rooted C-ABI vector projection matching Rust `marlin-gerbil-scheme`.

(import :std/test
        (only-in :clan/poo/object .get .o .ref)
        (only-in :tao3k/agentic-ai-lab/modules/graph-engine/obligation-graph
                 make-obligation make-obligation-graph)
        (only-in :tao3k/agentic-ai-lab/modules/graph-engine/delegation-fence
                 make-capability-envelope)
        (only-in :marlin-agent-core/graph-engine/interface GraphEngine))

(export marlin-graph-engine-governed-context-test)

(def (sample-obligation-graph has-defeater?: (has-defeater? #f))
  (let* ((ob-a (make-obligation "ob-a"
                                title: "Verify Context"
                                required-evidence-ids: '("ev-a")
                                active-defeaters: (if has-defeater? '("def-1") '())))
         (ob-apex (make-obligation "ob-apex"
                                   title: "Settle"
                                   required-evidence-ids: '("ev-a")
                                   depends-on: '("ob-a"))))
    (make-obligation-graph "g-ctx"
                           cut: 2
                           obligations: (list ob-a ob-apex)
                           evidence: '()
                           det-tasks: '())))

(def sample-org
  (string-append
   "* Mission :mission:\n"
   ":PROPERTIES:\n"
   ":ID: mission-ctx-1\n"
   ":END:\n"
   "Verify governed context assembly and KV-prefix binding.\n"))

(def marlin-graph-engine-governed-context-test
  (test-suite "Marlin GraphEngine x ai-agentic-context Context Governance"
    (test-case "assemble-governed-context binds live KV-prefix, L3 defeater floor, and Design 165 blocker overlay"
      (let* ((exports (.get GraphEngine core exports))
             (assemble (.ref exports 'assemble-governed-context))
             (admit-nono (.ref exports 'admit-graph-nono-sandbox))
             (make-cedar (.ref exports 'make-graph-cedar-handoff))
             (env (make-capability-envelope "marlin-owner"
                                            allowed-caps: '(context-read verify-build)
                                            denied-caps: '(host-egress)
                                            egress-allowed?: #f))
             (sandbox-ok (admit-nono has-untrusted-input?: #f
                                     has-private-data?: #t
                                     egress-endorsed?: #f
                                     lease-valid?: #t))
             (cedar-ok (make-cedar "pol-ctx-1"
                                   "permit(principal, action, resource);"
                                   "trunk-ctx"))
             (res (assemble "task-ctx-1"
                            "trunk-ctx"
                            "ob-apex"
                            (sample-obligation-graph has-defeater?: #t)
                            env
                            sample-org
                            cedar-handoff: cedar-ok
                            sandbox-admission: sandbox-ok
                            detour-mode: 'blocker-active))
             (vec (.ref res 'abi-vector)))
        (check (.ref res 'admitted?) => #t)
        (check (.ref res 'auto-archive-enabled?) => #f)
        (check (.ref res 'runtime-executed?) => #f)
        (check (vector-length vec) => 13)
        (check (vector-ref vec 0) => 1)
        (check (vector-ref vec 1) => #t)
        (check (vector-ref vec 2) => "task-ctx-1")
        (check (vector-ref vec 3) => 2)
        (check (> (string-length (vector-ref vec 4)) 0) => #t)
        (check (vector-ref vec 5) => 'L3)
        (check (vector-ref vec 6) => #t)
        (check (vector-ref vec 8) => #t)
        (check (vector-ref vec 9) => #t)
        (check (vector-ref vec 10) => 'blocker-active)
        (check (vector-ref vec 11) => #f)
        (check (vector-ref vec 12) => #f)))

    (test-case "governed-context vector enforces fail-closed L3 defeater floor, Lethal-Trifecta sandbox, and blocker auto-archive"
      (let ((project-ctx (.get GraphEngine core exports project-governed-context-vector)))
        ;; Active defeater below L3 -> rejected fail-closed.
        (let ((bad-defeater (project-ctx
                              '#("task-1" 1 "sha256:abc" L2 #t 0 #t #t mainline #t))))
          (check (vector-ref bad-defeater 1) => #f)
          (check (vector-ref bad-defeater 12) => "active-defeater-requires-l3-disclosure"))

        ;; Lethal-Trifecta sandbox denial -> rejected fail-closed.
        (let ((bad-sandbox (project-ctx
                             '#("task-1" 1 "sha256:abc" L1 #f 0 #f #t mainline #t))))
          (check (vector-ref bad-sandbox 1) => #f)
          (check (vector-ref bad-sandbox 12) => "sandbox-lethal-trifecta-denied"))

        ;; Invalid Cedar policy handoff -> rejected fail-closed.
        (let ((bad-cedar (project-ctx
                           '#("task-1" 1 "sha256:abc" L1 #f 0 #t #f mainline #t))))
          (check (vector-ref bad-cedar 1) => #f)
          (check (vector-ref bad-cedar 12) => "invalid-cedar-policy-handoff"))

        ;; BlockerActive detour with auto-archive enabled -> rejected fail-closed.
        (let ((bad-blocker (project-ctx
                             '#("task-1" 1 "sha256:abc" L1 #f 0 #t #t blocker-active #t))))
          (check (vector-ref bad-blocker 1) => #f)
          (check (vector-ref bad-blocker 12) => "blocker-detour-must-disable-auto-archive"))

        ;; Lethal-Trifecta live Nono profile check: untrusted + private + unendorsed egress -> denied!
        (let* ((admit-nono (.get GraphEngine core exports admit-graph-nono-sandbox))
               (egress-profile (.o bind-these: #f
                                   backend-kind: 'nono
                                   capabilities: '(process-run network-egress)
                                   egress-enabled?: #t))
               (trifecta (admit-nono profile: egress-profile
                                     has-untrusted-input?: #t
                                     has-private-data?: #t
                                     egress-endorsed?: #f
                                     lease-valid?: #t)))
          (check (.ref trifecta 'admitted?) => #f))
        (displayln "MARLIN-GRAPH-ENGINE-GOVERNED-CONTEXT-OK")))))
