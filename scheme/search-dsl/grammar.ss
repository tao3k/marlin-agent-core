;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Typed structured sub-syntax parser declared via `:gerbil-parser` (`deflanguage`).
;;; Keeps lightweight query/selector parsing in Scheme so ASP Rust stays a
;;; minimal, language-agnostic runtime search engine interface.
(import (only-in :gerbil-parser/language-support/grammar deflanguage)
        (only-in :gerbil-parser/language-support/entry
                 deflanguage-parser-loader
                 language-parser-entry-ref)
        (only-in :gerbil-parser/execution/artifact parse-artifact-ref))
(export search-selector-syntax
        search-selector-grammar
        search-selector-parser-ir
        search-selector-parser
        search-selector-language
        parse-search-selector
        search-selector-artifact-summary)

(deflanguage search-selector
  (syntax
    (lexical
     (root selector-query)
     (lex
      (whitespace Whitespace (whitespace+))
      (identifier Identifier (identifier))
      (punctuation Punctuation (literals ":" "::" "." "/" "-" "&" "|" "!" "(" ")"))
      (unknown Unknown (fallback)))
     (extras whitespace)
     (keywords)
     (recoveries
      (selector-term "MARLIN-SEARCH-SELECTOR-TERM" preserve-source))
     (conflicts reject)
     (case-insensitive #f)))
  (rules
    (selector-query
     (node SelectorQuery (field expr selector-expr)))
    (selector-expr
     (choice
      (prec left 10
            (node OrSelector
                  (seq
                   (field left selector-expr)
                   (literal "|")
                   (field right selector-expr))))
      (prec left 20
            (node AndSelector
                  (seq
                   (field left selector-expr)
                   (literal "&")
                   (field right selector-expr))))
      (prec right 30 not-selector)
      grouped-selector
      scoped-selector
      facet-selector
      bare-selector))
    (not-selector
     (prec right 30
           (node NotSelector
                 (seq
                  (literal "!")
                  (field operand selector-expr)))))
    (grouped-selector
     (node GroupedSelector
           (seq
            (literal "(")
            (field expr selector-expr)
            (literal ")"))))
    (scoped-selector
     (node ScopedSelector
           (seq
            (field namespace identifier)
            (literal "::")
            (field member identifier))))
    (facet-selector
     (node FacetSelector
           (seq
            (field facet identifier)
            (literal ":")
            (field value identifier))))
    (bare-selector
     (node BareSelector
           (field name identifier)))))

(deflanguage-parser-loader search-selector-language
  (grammar search-selector-syntax)
  (parse parse-search-selector)
  (slots metadata: '((language . "search-selector")
                     (version . "v1")
                     (contract . "marlin.search-selector.v1")
                     (grammar-format . concise-dsl))))

(def (search-selector-artifact-summary source)
  (let* ((artifact (parse-search-selector source))
         (status (parse-artifact-ref artifact 'status))
         (meta (language-parser-entry-ref search-selector-language 'metadata))
         (lang-pair (and (list? meta) (assq 'language meta)))
         (language (if (and lang-pair (string? (cdr lang-pair)))
                     (cdr lang-pair)
                     "search-selector"))
         (root (parse-artifact-ref artifact 'root))
         (diagnostics (parse-artifact-ref artifact 'diagnostics)))
    (vector 'parse-artifact-summary
            status
            language
            (if root (parse-artifact-ref root 'kind) #f)
            (length diagnostics))))
