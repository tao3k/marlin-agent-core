;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Regression suite for Marlin's modular `scheme/repair` module and its opt-in
;;; `+ast-patch` Yuanxu feature (`Marlin.repair.ast-patch`).
(import :std/test
        (only-in :clan/poo/object .get .ref)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports)
        (only-in :marlin-agent-core/search-dsl/objects SearchLanguageRegistryValue))
(export repair-test)

(def repair-test
  (test-suite "Marlin Modular Repair & +ast-patch Feature"

    (test-case "Yuanxu mounts Marlin.repair with validated core and +ast-patch feature"
      (let* ((rep (.get marlin-agent-core.init.exports Marlin repair))
             (patch (.get rep ast-patch)))
        (check (eq? (.get rep identity) 'marlin/repair) => #t)
        (check (eq? (.get (.get rep policy) identity) 'marlin/repair-policy) => #t)
        (let* ((repaired
                 ((.get rep read-and-repair)
                  "(find-impact \"SharedCfg\" path-prefix: \"crates/\" exclude-tests?: #t)"
                  SearchLanguageRegistryValue))
               (form (vector-ref repaired 0))
               (tags (vector-ref repaired 1)))
          (check (car form) => 'ascent-fragment)
          (check tags => '()))
        (check (procedure? (.get patch build-request)) => #t)
        (check (procedure? (.get patch validate-request)) => #t)
        (check (procedure? (.get patch from-style-finding)) => #t)
        (check (procedure? (.get patch project-vector)) => #t)))

    (test-case "+ast-patch builds, validates, lowers ast-style findings, and projects zero-JSON rooted vectors"
      (let* ((rep (.get marlin-agent-core.init.exports Marlin repair))
             (patch (.get rep ast-patch))
             (req ((.get patch build-request)
                   '((path . "crates/marlin-agent-tooling/src/search_tool.rs")
                     (op . replace-item)
                     (target . "fn#execute_resident_search_tool")
                     (snippet . "pub fn execute_resident_search_tool() {}")
                     (language . "rust")
                     (expected-preimage-hash . "sha256:abcdef0123456789")
                     (dry-run? . #t))))
             (vec ((.get patch project-vector) req))
             (style-finding
               '("RUST-MOD-R001" "propose-ast-patch" "e1"
                 "crates/foo/src/mod.rs" "helper_fn" 18))
             (from-style
               ((.get patch from-style-finding)
                style-finding
                "fn#helper_fn"
                '((op . remove-node)
                  (expected-preimage-hash . "sha256:deadbeef")))))
        (check ((.get patch validate-request) req) => #t)
        (check (vector-length vec) => 10)
        (check (vector-ref vec 0) => 'ast-patch-request)
        (check (vector-ref vec 1) => "crates/marlin-agent-tooling/src/search_tool.rs")
        (check (vector-ref vec 4) => "replace_item")
        (check (vector-ref vec 5) => "fn#execute_resident_search_tool")
        (check (vector-ref vec 8) => #t)
        (check ((.get patch validate-request) from-style) => #t)
        (check (.ref from-style 'op-wire) => "remove_node")
        (check (.ref from-style 'rule-id) => "RUST-MOD-R001")))

    (test-case "+ast-patch fails closed on path traversal, missing target/snippet, and pre-gen-contract findings"
      (let* ((rep (.get marlin-agent-core.init.exports Marlin repair))
             (patch (.get rep ast-patch)))
        (check-exception
          ((.get patch build-request)
           '((path . "../escape.rs") (op . remove-node) (target . "fn#x")))
          true)
        (check-exception
          ((.get patch build-request)
           '((path . "/etc/passwd") (op . remove-node) (target . "fn#x")))
          true)
        (check-exception
          ((.get patch build-request)
           '((path . "crates/a/src/lib.rs") (op . replace-item) (target . "fn#x")))
          true)
        (check-exception
          ((.get patch from-style-finding)
           '("RUST-AGENT-DATA-MEMBERSHIP-029" "pre-gen-contract" "e2"
             "crates/a/src/lib.rs" "scan" 40)
           "fn#scan"
           "")
          true)))))
