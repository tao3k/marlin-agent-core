;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Validated publication for Marlin's unified `:gerbil-ascent` + `:gerbil-parser`
;;; + POO Search DSL module.
(import (only-in :clan/poo/mop .new/checked)
        "./types.ss"
        "./objects.ss"
        "./extract.ss"
        "./funcs.ss")
(export SearchDslScope)

(def SearchDslScope
  (.new/checked SearchDslScopeContract
    identity: 'marlin/search-dsl
    registry: SearchLanguageRegistryValue
    read-query: (lambda (text) (read-ascent-search-form text SearchLanguageRegistryValue))
    repair-query: (lambda (text) (repair-ascent-search-form text SearchLanguageRegistryValue))
    compile-prefilter: (lambda (text) (compile-search-dsl-prefilter text SearchLanguageRegistryValue))
    evaluate-edb: (lambda (text edb) (evaluate-search-dsl-over-edb text edb SearchLanguageRegistryValue))
    extract-facts: extract-open-edb-facts-batch
    completions: (lambda (text) (search-dsl-completions text SearchLanguageRegistryValue))
    tool-description: (lambda () (search-dsl-tool-description SearchLanguageRegistryValue))
    parse-selector: parse-search-dsl-selector
    conformance-pack: (lambda (lang) (project-language-conformance-pack lang SearchLanguageRegistryValue))
    all-conformance-packs: (lambda () (project-all-conformance-packs SearchLanguageRegistryValue))
    language-json-schema: (lambda (lang) (project-language-json-schema lang SearchLanguageRegistryValue))))
