;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; POO prototypes and sealed search plan declarations for Marlin's native
;;; ASP search engine bridge. Plans compose as real Gerbil Scheme values and
;;; cross the Rust boundary as rooted Scheme vectors (no text serialization).
(import (only-in :clan/poo/object .def .mix seal-object!))
(export SearchLeaf.
        RgLeaf.
        TantivyLeaf.
        TopologyLeaf.
        SyntaxLeaf.
        NativeSyntaxLeaf.
        GraphLeaf.
        ChainPlan.
        IntersectPlan.
        SearchPlan.
        DefaultSearchPlanValue)

(.def SearchLeaf.
  kind: 'leaf)

(.def RgLeaf.
  kind: 'rg
  argv: '())

(.def TantivyLeaf.
  kind: 'tantivy
  pattern: "")

(.def TopologyLeaf.
  kind: 'topology
  member-kind: "file"
  exact-path: #f
  path-prefix: #f
  extension: #f
  path-glob: #f)

(.def SyntaxLeaf.
  kind: 'syntax
  producer: 'rust
  pattern: "")

(.def NativeSyntaxLeaf.
  kind: 'native-syntax
  selector: "")

(.def GraphLeaf.
  kind: 'graph
  language: 'pgql
  argv: '())

(.def ChainPlan.
  kind: 'chain
  children: '())

(.def IntersectPlan.
  kind: 'intersect
  children: '())

(.def SearchPlan.
  workspace: #f
  languages: '()
  documents: '()
  composition: #f)

(.def DefaultRgLeaf.
  argv: '("-n" "MarlinConvergedSearchEngine"))

(.def DefaultTantivyLeaf.
  pattern: "title:\"converged search\"^2 OR body:SearchEngineStoragePort")

(.def DefaultTopologyLeaf.
  path-prefix: "crates/"
  extension: "rs")

(.def DefaultGraphLeaf.
  language: 'pgql
  argv: '("MATCH (a)-[r]->(b) RETURN a, r, b"))

(def DefaultRgLeafValue
  (seal-object! (.mix DefaultRgLeaf. RgLeaf.)))

(def DefaultTantivyLeafValue
  (seal-object! (.mix DefaultTantivyLeaf. TantivyLeaf.)))

(def DefaultTopologyLeafValue
  (seal-object! (.mix DefaultTopologyLeaf. TopologyLeaf.)))

(def DefaultGraphLeafValue
  (seal-object! (.mix DefaultGraphLeaf. GraphLeaf.)))

(.def DefaultIntersect.
  children: (list DefaultRgLeafValue DefaultTantivyLeafValue DefaultTopologyLeafValue))

(def DefaultIntersectValue
  (seal-object! (.mix DefaultIntersect. IntersectPlan.)))

(.def DefaultChain.
  children: (list DefaultIntersectValue DefaultGraphLeafValue))

(def DefaultChainValue
  (seal-object! (.mix DefaultChain. ChainPlan.)))

(.def DefaultSearchPlan.
  languages: '(rust)
  composition: DefaultChainValue)

(def DefaultSearchPlanValue
  (seal-object! (.mix DefaultSearchPlan. SearchPlan.)))
