;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Scheme TUI behavior and projection tests for Marlin's Modern Helm (`scheme/ui-helm`).
(import :std/test
        (only-in :clan/poo/object .get .o)
        (only-in :std/string/misc string-contains)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports)
        (only-in :marlin-agent-core/ui-helm/config UiHelmScope)
        (only-in :marlin-agent-core/ui/helm-workbench make-marlin-helm-workbench-package)
        (only-in :marlin-agent-core/ui-helm/funcs
                 helm-open-session
                 helm-parse-input
                 helm-update-input
                 helm-apply-results
                 helm-move-selection
                 helm-toggle-mark
                 helm-next-source
                 helm-toggle-preview
                 helm-toggle-actions
                 helm-dispatch-action
                 helm-visible-candidates
                 helm-selected-candidate
                 helm-render-lines
                 helm-project-ui-elements
                 make-helm-candidate))

(export marlin-ui-helm-test)

(def (sample-candidates)
  (list
    (make-helm-candidate
      "hit-1"
      "crates/marlin-agent-search-engine/src/engine.rs:14"
      source-id: 'search
      path: "crates/marlin-agent-search-engine/src/engine.rs"
      line: 14
      language: "rust"
      axes: '("rg" "tantivy")
      score: 95
      annotation: "rust · rg+tantivy · 95"
      preview-lines: '("14: pub fn marlin_search_engine("
                       "15:   org_port: Arc<dyn OrgDocumentProjectionPort>,"))
    (make-helm-candidate
      "hit-2"
      "crates/marlin-agent-search-engine/src/storage_port.rs:84"
      source-id: 'search
      path: "crates/marlin-agent-search-engine/src/storage_port.rs"
      line: 84
      language: "rust"
      axes: '("rg")
      score: 80
      annotation: "rust · rg · 80"
      preview-lines: '("84: edge_id: scoped_topology_edge_id(...)"))
    (make-helm-candidate
      "buf-1"
      "*conversation*"
      source-id: 'buffers
      score: 60
      annotation: "buffer · active")
    (make-helm-candidate
      "org-1"
      "dev/sysml/search-engine.sysml:MAR-SRCH-R04"
      source-id: 'org
      path: "dev/sysml/search-engine.sysml"
      score: 70
      annotation: "org · sysml")))

(def marlin-ui-helm-test
  (test-suite "Marlin Modern Helm (scheme/ui-helm)"
    (test-case "two-stage Consult + Orderless input parser and native Playbook projection"
      (let* ((parsed (helm-parse-input
                       "@lang:rust @doc:org @prefix:crates/ @ext:rs execute_admitted #storage !test"))
             (plan-vec ((.get UiHelmScope compile-plan-vector)
                        "@lang:rust @doc:org @prefix:crates/ @ext:rs execute_admitted #storage !test")))
        (check (.get parsed async-pattern) => "execute_admitted")
        (check (.get parsed local-tokens) => '("storage" "!test"))
        (check (.get parsed languages) => '("rust"))
        (check (.get parsed documents) => '("org"))
        (check (.get parsed path-prefix) => "crates/")
        (check (.get parsed extension) => "rs")
        ;; Rooted C-ABI vector shape: #(workspace languages documents composition)
        (check (vector? plan-vec) => #t)
        (check (vector-length plan-vec) => 4)
        (check (vector-ref plan-vec 1) => '#(rust))
        (check (vector-ref plan-vec 2) => '#(org))
        (check (vector-ref (vector-ref plan-vec 3) 0) => 'intersect)))

    (test-case "epoch fencing drops stale async search results fail-closed"
      (let* ((s0 (helm-open-session viewport-height: 2))
             (s1 (helm-update-input s0 "@lang:rust marlin_search_engine"))
             (epoch1 (.get s1 epoch))
             (s2 (helm-update-input s1 "@lang:rust storage_port"))
             (epoch2 (.get s2 epoch))
             ;; Late response from epoch1 must be ignored!
             (s3 (helm-apply-results s2 epoch1 "blake3-256:stale" (sample-candidates)))
             ;; Fresh response from epoch2 is accepted!
             (s4 (helm-apply-results s3 epoch2 "blake3-256:fresh1234567890" (sample-candidates))))
        (check (> epoch2 epoch1) => #t)
        (check (length (.get s3 candidates)) => 0)
        (check (.get s3 generation-digest) => #f)
        (check (length (.get s4 candidates)) => 4)
        (check (.get s4 generation-digest) => "blake3-256:fresh1234567890")))

    (test-case "local Orderless filtering preserves epoch without re-triggering async search"
      (let* ((s0 (helm-open-session viewport-height: 3))
             (s1 (helm-update-input s0 "@lang:rust marlin"))
             (epoch-before (.get s1 epoch))
             (s2 (helm-apply-results s1 epoch-before "blake3-256:gen1" (sample-candidates)))
             (s3 (helm-update-input s2 "@lang:rust marlin #crates !engine.rs")))
        ;; Only local filter changed -> epoch stays identical (zero redundant Rust search!)
        (check (.get s3 epoch) => epoch-before)
        (check (length (.get s3 candidates)) => 1)
        (check (.get (car (.get s3 candidates)) id) => "hit-2")))

    (test-case "virtual viewport scrolling, multi-source cycling, marks, Embark actions, and TUI render"
      (let* ((s0 (helm-open-session viewport-height: 2))
             (s1 (helm-apply-results s0 (.get s0 epoch) "blake3-256:abcdef0123456789" (sample-candidates)))
             (s2 (helm-toggle-mark s1))
             (s3 (helm-move-selection s2 2))
             (s4 (helm-toggle-mark s3))
             (s5 (helm-toggle-actions s4))
             (lines (helm-render-lines s5))
             (joined (apply string-append (map (lambda (l) (string-append l "\n")) lines)))
             (s6 (helm-dispatch-action s5 'export-context))
             (receipt (.get s6 last-action-receipt))
             (s7 (helm-next-source s6)))
        ;; Virtual viewport caps visible rows to viewport-height = 2
        (check (length (helm-visible-candidates s3)) => 2)
        (check (.get (helm-selected-candidate s3) id) => "buf-1")
        (check (and (string-contains joined "HELM [all]") #t) => #t)
        (check (and (string-contains joined "marked=2") #t) => #t)
        (check (and (string-contains joined "--- Actions:") #t) => #t)
        ;; Embark action receipt preserves marked ids and generation digest
        (check (.get receipt action-id) => 'export-context)
        (check (.get receipt target-ids) => '("hit-1" "buf-1"))
        (check (.get receipt generation-digest) => "blake3-256:abcdef0123456789")
        ;; Cycling source filters to 'search only
        (check (.get s7 active-source) => 'search)
        (check (length (.get s7 candidates)) => 2)
        ;; Preview toggle works cleanly
        (check (.get (helm-toggle-preview s7) layout) => 'bottom-list)))

    (test-case "TUI workbench exposes SPC s keymap, helm command, declarative UI elements, and *Helm* package buffer"
      (let* ((ui (.get marlin-agent-core.init.exports Marlin ui))
             (base-keys (vector->list ((.get ui keymap-entries))))
             (s0 (helm-open-session viewport-height: 2))
             (s1 (helm-apply-results s0 (.get s0 epoch) "blake3-256:ui0123456789" (sample-candidates)))
             (ui-tree (helm-project-ui-elements s1))
             (pkg-ui (make-marlin-helm-workbench-package s1))
             (wb ((.get pkg-ui workbench)
                  (.o model: "gemini-2.5-pro"
                      session: "helm-workbench"
                      contributions: '()
                      extensions: '())))
             (projected ((.get pkg-ui project-workbench) wb))
             (pkg-keys (vector->list ((.get pkg-ui keymap-entries))))
             (frame0 ((.get pkg-ui make-frame)))
             (frame1 ((.get pkg-ui select-buffer) frame0 'helm-snapshot))
             (proj-frame ((.get pkg-ui project-frame) frame1)))
        ;; Built-in SPC s launches live interactive Helm
        (check (and (member '#(normal #(SPC s) helm) base-keys) #t) => #t)
        ;; Declarative UI element tree starts with 'rows
        (check (vector? ui-tree) => #t)
        (check (vector-ref ui-tree 0) => 'rows)
        ;; Package workbench projects cleanly into Ratatui native vector
        (check (vector? projected) => #t)
        ;; Buffer switching activates 'helm-snapshot in main window
        (check (vector-ref (vector-ref (vector-ref proj-frame 1) 0) 1) => 'helm-snapshot)
        ;; Package keybinding SPC f h binds cleanly without colliding with SPC s
        (check (and (member '#(normal #(SPC f h) buffer-helm-snapshot) pkg-keys) #t) => #t)
        ;; Scheme owns Helm index exclusions and bottom-split Emacs window policy
        (let ((idx-cfg ((.get UiHelmScope index-config-vector)))
              (win-pol ((.get UiHelmScope window-policy-vector))))
          (check (vector? idx-cfg) => #t)
          (check (vector-length idx-cfg) => 4)
          (check (and (member "target" (vector->list (vector-ref idx-cfg 0))) #t) => #t)
          (check (and (member ".gerbil" (vector->list (vector-ref idx-cfg 0))) #t) => #t)
          (check (vector-ref idx-cfg 1) => 2097152)
          (check (vector? win-pol) => #t)
          (check (vector-ref win-pol 0) => 'bottom-split)
          (check (vector-ref win-pol 4) => #t)
          (check (vector-ref win-pol 5) => #t)
          (check (vector-ref win-pol 6) => #t))))))
