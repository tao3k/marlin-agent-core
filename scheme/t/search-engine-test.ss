;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; -*- Gerbil -*-
;;; Verify Marlin's real Gerbil Scheme search-engine module composition and
;;; rooted vector projection for `crates/marlin-agent-search-engine` + ASP.

(import :std/test
        (only-in :clan/poo/object .get .o seal-object!)
        (only-in :marlin-agent-core/search-engine/config SearchEngineScope)
        (only-in :marlin-agent-core/search-engine/objects SearchPlan. TantivyLeaf.)
        (only-in :marlin-agent-core/search-engine/funcs
                 rg-leaf tantivy-leaf topology-leaf syntax-leaf
                 native-syntax-leaf graph-leaf chain-plan intersect-plan
                 make-search-plan))

(export marlin-search-engine-test)

(def search-engine-scope SearchEngineScope)

(def marlin-search-engine-test
  (test-suite "Marlin SearchEngine Scheme Module & Native ABI Projection"

    (test-case "SearchEngineScope publishes identity and default converged vector"
      (let ((projected ((.get search-engine-scope project))))
        (check (.get search-engine-scope identity) => 'marlin/search-engine)
        (check (vector? projected) => #t)
        (check (vector-length projected) => 4)
        (check (vector-ref projected 0) => #f)
        (check (vector-ref projected 1) => #(rust))
        (check (vector-ref projected 2) => #())
        (check (vector-ref (vector-ref projected 3) 0) => 'chain)))

    (test-case "functional constructors compose full multi-axis progressive search plan"
      (let* ((plan (make-search-plan
                     (chain-plan
                       (intersect-plan
                         (rg-leaf "-n" "MarlinConvergedSearchEngine")
                         (tantivy-leaf "title:\"converged search\"^2 OR body:OrgDocumentProjectionPort")
                         (topology-leaf member-kind: "file"
                                        path-prefix: "crates/"
                                        extension: "rs"))
                       (syntax-leaf 'rust "(function_item name: (identifier) @item)")
                       (native-syntax-leaf "rust://crates/marlin-agent-search-engine/src/lib.rs#item/function/admit")
                       (graph-leaf 'pgql "MATCH (a)-[r]->(b) RETURN a, r, b"))
                     workspace: "marlin-main"
                     languages: '(rust python)
                     documents: '(org)))
             (projected ((.get search-engine-scope project-plan) plan)))
        (check projected
               => #("marlin-main"
                    #(rust python)
                    #(org)
                    #(chain
                      #(#(intersect
                          #(#(rg #("-n" "MarlinConvergedSearchEngine"))
                            #(tantivy "title:\"converged search\"^2 OR body:OrgDocumentProjectionPort")
                            #(topology "file" #f "crates/" "rs" #f)))
                        #(syntax rust "(function_item name: (identifier) @item)")
                        #(native-syntax "rust://crates/marlin-agent-search-engine/src/lib.rs#item/function/admit")
                        #(graph pgql #("MATCH (a)-[r]->(b) RETURN a, r, b"))))))))

    (test-case "POO prototype inheritance composes custom search plans cleanly"
      (let* ((custom-plan
               (seal-object!
                 (.o (:: @ SearchPlan.)
                     workspace: #f
                     languages: '(rust)
                     documents: '()
                     composition:
                     (.o (:: @ TantivyLeaf.)
                         pattern: "title:\"converged search\"^2 OR body:SearchEngineStoragePort"))))
             (projected ((.get search-engine-scope project-plan) custom-plan)))
        (check projected
               => #(#f #(rust) #() #(tantivy "title:\"converged search\"^2 OR body:SearchEngineStoragePort")))))))
