;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Exercise native projection limits with synthetic Scheme values. This does
;;; not invoke Orgize's own earlier source-graph size limit.
(import (only-in :clan/poo/object .o)
        (only-in :marlin-agent-core/c-abi/context
                 marlin-context-org-source-project marlin-context-org-source-fields
                 marlin-context-org-contract-project))

(def (rejected? action)
  (with-catch (lambda (_) #t) (lambda () (action) #f)))
(def record (.o id: 0 parent: #f kind: 'headline start: 0 end: 0))
(def (source count)
  (.o source: "" graph: (.o records: (make-list count record)
                           field-of: (lambda (_record _name) #f))))
(def (source-with-records records)
  (.o source: "" graph: (.o records: records field-of: (lambda (_record _name) #f))))
(def (contract count)
  (let (node (.o identity: "a" propertyId: "a" elementKind: 'headline byteStart: 0))
    (.o identity: "source" generation: 1 source: "" source-digest: "digest"
        semantic-digest: "digest" contract-identity: "contract" contract-revision: "1"
        session-identity: #f parser-identity: "parser" nodes: (make-list count node))))

(unless (and (= (vector-length (vector-ref (marlin-context-org-source-project (source 4096)) 1)) 4096)
             (equal? (marlin-context-org-source-fields (vector (source 4096) #())) #())
             (rejected? (lambda () (marlin-context-org-source-project (source 4097))))
             (rejected? (lambda () (marlin-context-org-source-fields (vector (source 4097) #()))))
             (rejected? (lambda () (marlin-context-org-source-project (source-with-records (cons record 'invalid)))))
             (= (vector-length (vector-ref (marlin-context-org-contract-project (contract 32)) 9)) 32)
             (rejected? (lambda () (marlin-context-org-contract-project (contract 33)))))
  (error "Context Org native projection record bounds failed"))
(displayln "CONTEXT-ORG-BOUNDARY-OK")
