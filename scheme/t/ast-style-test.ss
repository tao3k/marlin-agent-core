;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Regression suite for Marlin's Frontier 1 Machine-Native Relational Coding
;;; Style Engine (`scheme/ast-style`, RFC 40.185 / ASP RFC 10.35).
(import :std/test
        (only-in :std/string/misc string-contains)
        (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports)
        (only-in :marlin-agent-core/ast-style/funcs
                 evaluate-coding-style-over-edb
                 project-symbolic-style-contract
                 compute-style-cost-tuple
                 style-cost<?
                 partition-style-findings
                 ast-style-eval-edb-vector
                 ast-style-project-contract-vector
                 ast-style-cost-less-vector))
(export ast-style-test)

(def sample-verbose-edb
  '((file . ((1 "rust" "crates/demo/src/mod.rs")
             (2 "rust" "crates/demo/src/engine.rs")
             (3 "scheme" "scheme/demo/funcs.ss")))
    (entity . (("r1" 1 "rust"   "function" "fn_item" "inline_helper"   12)
               ("r2" 2 "rust"   "function" "fn_item" "open_session"    24)
               ("r3" 2 "rust"   "function" "fn_item" "collect_active"  48)
               ("r4" 2 "rust"   "function" "fn_item" "loop_with_break" 65)
               ("r5" 2 "rust"   "function" "fn_item" "map_option"      82)
               ("s1" 3 "scheme" "item"     "poo_def" "MisplacedObj."   18)))
    (edge . ())
    (attr . (("r1" "in_mod_rs"                "true")
             ("r2" "visibility"               "pub")
             ("r2" "primitive_semantic_param" "true")
             ("r3" "mut_vec_push_loop"        "true")
             ;; r4 has early_exit=true -> stratified negation MUST NOT elevate!
             ("r4" "mut_vec_push_loop"        "true")
             ("r4" "early_exit"               "true")
             ("r5" "option_map_match"         "true")
             ("s1" "in_funcs_ss"              "true")))))

(def sample-elevated-edb
  '((file . ((2 "rust" "crates/demo/src/engine.rs")))
    (entity . (("r3" 2 "rust" "function" "fn_item" "collect_active" 48)
               ("r5" 2 "rust" "function" "fn_item" "map_option"     82)))
    (edge . ())
    (attr . ())))

(def ast-style-test
  (test-suite "Marlin Frontier 1 :gerbil-ascent Relational Coding Style Engine"

    (test-case "Yuanxu mounts Marlin.ast-style with validated scope contract"
      (let ((style (.get marlin-agent-core.init.exports Marlin ast-style)))
        (check (eq? (.get style identity) 'marlin/ast-style) => #t)))

    (test-case "Real :gerbil-ascent fixpoint detects Rust + Scheme POO style findings and respects stratified negation"
      (let* ((findings (evaluate-coding-style-over-edb sample-verbose-edb))
             (rule-ids (map car findings))
             (parts (partition-style-findings findings))
             (auto-ids (map (lambda (r) (list-ref r 2)) (vector-ref parts 0)))
             (propose-ids (map (lambda (r) (list-ref r 2)) (vector-ref parts 1))))
        ;; Must detect RUST-MOD-R001, RUST-AGENT-API-TYPE-012, RUST-AGENT-ITER-IMPL-026,
        ;; RUST-AGENT-CFG-PUBLIC-015, and SCM-POO-001!
        (check (and (member "RUST-MOD-R001" rule-ids) #t) => #t)
        (check (and (member "RUST-AGENT-API-TYPE-012" rule-ids) #t) => #t)
        (check (and (member "RUST-AGENT-ITER-IMPL-026" rule-ids) #t) => #t)
        (check (and (member "RUST-AGENT-CFG-PUBLIC-015" rule-ids) #t) => #t)
        (check (and (member "SCM-POO-001" rule-ids) #t) => #t)
        ;; Class-1 auto-elevate includes r3 and r5, NEVER r4 (stratified negation early_exit=true)!
        (check (and (member "r3" auto-ids) #t) => #t)
        (check (and (member "r5" auto-ids) #t) => #t)
        (check (member "r4" auto-ids) => #f)
        ;; Class-2 architectural proposals include r1, r2, s1!
        (check (and (member "r1" propose-ids) #t) => #t)
        (check (and (member "r2" propose-ids) #t) => #t)
        (check (and (member "s1" propose-ids) #t) => #t)))

    (test-case "Lexicographic StyleCostTuple proves strict monotone cost descent after elevation"
      (let* ((cost-before (compute-style-cost-tuple (evaluate-coding-style-over-edb sample-verbose-edb)))
             (cost-after (compute-style-cost-tuple (evaluate-coding-style-over-edb sample-elevated-edb))))
        (check cost-before => '(3 0 2))
        (check cost-after => '(0 0 0))
        (check (style-cost<? cost-after cost-before) => #t)
        (check (style-cost<? cost-before cost-after) => #f)))

    (test-case "Pi_LLM projects compact machine-native mathematical contract directly from :gerbil-ascent rules"
      (let ((rust-contract
              (project-symbolic-style-contract
                "rust"
                "crates/demo/src/engine.rs#fn:collect_active"
                '("AstProviderId" "AstWorkspacePath")))
            (scheme-contract
              (project-symbolic-style-contract
                "scheme"
                "scheme/demo/objects.ss#poo:Engine."
                '("CodingStyleRegistry."))))
        (check (and (string-contains rust-contract "IN_SCOPE_TYPES: AstProviderId, AstWorkspacePath")
                    (string-contains rust-contract "[RUST-AGENT-ITER-IMPL-026]")
                    (string-contains rust-contract "⇝")
                    (string-contains rust-contract "COST_LATTICE:")
                    #t)
               => #t)
        (check (and (string-contains scheme-contract "[SCM-POO-001]")
                    (string-contains scheme-contract "[SCM-POO-003]")
                    (string-contains scheme-contract "(s := E)")
                    #t)
               => #t)))

    (test-case "Rooted C-ABI ast-style entrypoints round-trip native Scheme vectors with zero JSON"
      (let* ((alist->vec (lambda (edb key)
                           (list->vector
                             (map list->vector (cdr (assq key edb))))))
             (edb-vec (vector (alist->vec sample-verbose-edb 'file)
                              (alist->vec sample-verbose-edb 'entity)
                              (alist->vec sample-verbose-edb 'edge)
                              (alist->vec sample-verbose-edb 'attr)))
             (res (ast-style-eval-edb-vector edb-vec))
             (findings-vec (vector-ref res 0))
             (cost-vec (vector-ref res 1))
             (contract (ast-style-project-contract-vector
                         (vector "rust"
                                 "crates/demo/src/engine.rs#fn:collect_active"
                                 (vector "AstProviderId" "AstWorkspacePath")))))
        (check (vector? findings-vec) => #t)
        (check (= (vector-length findings-vec) 5) => #t)
        (check cost-vec => #(3 0 2))
        (check (ast-style-cost-less-vector (vector #(0 0 0) cost-vec)) => #t)
        (check (ast-style-cost-less-vector (vector cost-vec #(0 0 0))) => #f)
        (check (and (string-contains contract "[RUST-AGENT-ITER-IMPL-026]")
                    (string-contains contract "AstProviderId, AstWorkspacePath")
                    #t)
               => #t)))))
