;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; -*- Gerbil -*-
;;; Boundary: Type contracts for Marlin's GraphEngine integration.

(import (only-in :clan/poo/object .ref .slot? object?)
        (only-in :clan/poo/type Type. define-type))

(export MarlinGraphEngineDispatchResult
        MarlinGraphEngineSqliteTursoContract)

(define-type (MarlinGraphEngineDispatchResult @ Type.)
  identity: 'marlin/graph-engine-dispatch-result
  .element?: (lambda (value)
               (and (object? value)
                    (.slot? value 'admitted?)
                    (boolean? (.ref value 'admitted?))
                    (.slot? value 'errors)
                    (list? (.ref value 'errors)))))

(define-type (MarlinGraphEngineSqliteTursoContract @ Type.)
  identity: 'marlin/graph-engine-sqlite-turso-feature
  .element?: (lambda (scope)
               (and (object? scope)
                    (.slot? scope 'sqlite-turso)
                    (object? (.ref scope 'sqlite-turso)))))
