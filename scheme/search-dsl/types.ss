;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Type contracts for Marlin's unified `:gerbil-ascent` + `:gerbil-parser` +
;;; POO Search DSL (`scheme/search-dsl`).
(import (only-in :clan/poo/object .cc object?)
        (only-in :clan/poo/type Exactly Fun Record String Type.))
(export SearchDslAny
        SearchDslVector
        SearchLanguageRegistryObject
        SearchDslScopeContract)

(def SearchDslAny (.cc Type. '.element? (lambda (_) #t)))
(def SearchDslVector (.cc Type. '.element? vector?))
(def SearchLanguageRegistryObject (.cc Type. '.element? object?))

(def SearchDslScopeContract
  (.cc (Record identity: [(Exactly 'marlin/search-dsl)]
               registry: [SearchLanguageRegistryObject]
               read-query: [(Fun SearchDslAny <- String)]
               repair-query: [(Fun SearchDslVector <- String)]
               compile-prefilter: [(Fun SearchDslVector <- String)]
               evaluate-edb: [(Fun SearchDslAny <- String SearchDslAny)]
               extract-facts: [(Fun SearchDslVector <- SearchDslVector)]
               completions: [(Fun SearchDslVector <- String)]
               tool-description: [(Fun String <-)]
               parse-selector: [(Fun SearchDslAny <- String)]
               conformance-pack: [(Fun SearchDslAny <- SearchDslAny)]
               all-conformance-packs: [(Fun SearchDslAny <-)])
       identity: 'marlin/search-dsl))
