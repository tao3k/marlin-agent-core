;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Rooted Scheme C-ABI entrypoints for Marlin's Frontier 1 Machine-Native
;;; Relational Coding Style Engine (`scheme/ast-style`, RFC 40.185 / ASP RFC 10.35).
;;; Zero JSON across the native ABI boundary.
(import (only-in :marlin-agent-core/c-abi/roots marlin-rooted-call)
        (only-in :marlin-agent-core/ast-style/funcs
                 ast-style-eval-edb-vector
                 ast-style-project-contract-vector
                 ast-style-cost-less-vector))
(export marlin-ast-style-eval-edb
        marlin-ast-style-project-contract
        marlin-ast-style-cost-less)

(def (marlin-ast-style-eval-edb input)
  (ast-style-eval-edb-vector input))

(def (marlin-ast-style-project-contract input)
  (ast-style-project-contract-vector input))

(def (marlin-ast-style-cost-less input)
  (ast-style-cost-less-vector input))

(begin-foreign
  (c-define (ast-style-eval-edb root) (int64) int64
    "marlin_ast_style_eval_edb" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/ast-style#marlin-ast-style-eval-edb))
  (c-define (ast-style-project-contract root) (int64) int64
    "marlin_ast_style_project_contract" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/ast-style#marlin-ast-style-project-contract))
  (c-define (ast-style-cost-less root) (int64) int64
    "marlin_ast_style_cost_less" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/ast-style#marlin-ast-style-cost-less)))
