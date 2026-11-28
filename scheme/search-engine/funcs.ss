;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure functional constructors and rooted C-ABI projections for Marlin search
;;; plans. Every projected value uses only symbols, strings, booleans and
;;; vectors so Rust's `marlin-agent-search-engine` decodes directly from
;;; `gerbil_scheme::RootedSchemeObject` without text serialization.
(import (only-in :clan/poo/object .get .o seal-object!)
        "./objects.ss")
(export rg-leaf
        tantivy-leaf
        topology-leaf
        syntax-leaf
        native-syntax-leaf
        graph-leaf
        chain-plan
        intersect-plan
        make-search-plan
        project-search-composition
        project-search-plan)

(def (rg-leaf . argv)
  (seal-object!
    (.o (:: @ RgLeaf.)
        (argv := argv))))

(def (tantivy-leaf pattern)
  (seal-object!
    (.o (:: @ TantivyLeaf.)
        (pattern := pattern))))

(def (topology-leaf
       member-kind: (member-kind "file")
       exact-path: (exact-path #f)
       path-prefix: (path-prefix #f)
       extension: (extension #f)
       path-glob: (path-glob #f))
  (seal-object!
    (.o (:: @ TopologyLeaf.)
        (member-kind := member-kind)
        (exact-path := exact-path)
        (path-prefix := path-prefix)
        (extension := extension)
        (path-glob := path-glob))))

(def (syntax-leaf producer pattern)
  (seal-object!
    (.o (:: @ SyntaxLeaf.)
        (producer := producer)
        (pattern := pattern))))

(def (native-syntax-leaf selector)
  (seal-object!
    (.o (:: @ NativeSyntaxLeaf.)
        (selector := selector))))

(def (graph-leaf language . argv)
  (seal-object!
    (.o (:: @ GraphLeaf.)
        (language := language)
        (argv := argv))))

(def (chain-plan . children)
  (seal-object!
    (.o (:: @ ChainPlan.)
        (children := children))))

(def (intersect-plan . children)
  (seal-object!
    (.o (:: @ IntersectPlan.)
        (children := children))))

(def (make-search-plan
       composition
       workspace: (workspace #f)
       languages: (languages '())
       documents: (documents '()))
  (seal-object!
    (.o (:: @ SearchPlan.)
        (workspace := workspace)
        (languages := languages)
        (documents := documents)
        (composition := composition))))

(def (require-symbol-or-string value label)
  (cond
    ((symbol? value) value)
    ((string? value) (string->symbol value))
    (else (error "expected symbol or string" label value))))

(def (project-string-vector items label)
  (unless (list? items)
    (error "expected string list" label items))
  (list->vector
    (map (lambda (item)
           (unless (string? item)
             (error "expected string entry" label item))
           item)
         items)))

(def (project-optional-string value label)
  (cond
    ((not value) #f)
    ((string? value) value)
    (else (error "expected #f or string" label value))))

(def (project-search-composition node)
  (let ((kind (.get node kind)))
    (case kind
      ((chain)
       (vector 'chain
               (list->vector
                 (map project-search-composition (.get node children)))))
      ((intersect)
       (vector 'intersect
               (list->vector
                 (map project-search-composition (.get node children)))))
      ((rg)
       (vector 'rg (project-string-vector (.get node argv) 'rg-argv)))
      ((tantivy)
       (let ((pattern (.get node pattern)))
         (unless (string? pattern)
           (error "tantivy pattern must be a string" pattern))
         (vector 'tantivy pattern)))
      ((topology)
       (let ((member-kind (.get node member-kind)))
         (unless (string? member-kind)
           (error "topology member-kind must be a string" member-kind))
         (vector 'topology
                 member-kind
                 (project-optional-string (.get node exact-path) 'exact-path)
                 (project-optional-string (.get node path-prefix) 'path-prefix)
                 (project-optional-string (.get node extension) 'extension)
                 (project-optional-string (.get node path-glob) 'path-glob))))
      ((syntax)
       (let ((producer (require-symbol-or-string (.get node producer) 'syntax-producer))
             (pattern (.get node pattern)))
         (unless (string? pattern)
           (error "syntax pattern must be a string" pattern))
         (vector 'syntax producer pattern)))
      ((native-syntax)
       (let ((selector (.get node selector)))
         (unless (string? selector)
           (error "native-syntax selector must be a string" selector))
         (vector 'native-syntax selector)))
      ((graph)
       (let ((language (require-symbol-or-string (.get node language) 'graph-language)))
         (vector 'graph language (project-string-vector (.get node argv) 'graph-argv))))
      (else
       (error "unsupported search composition node kind" kind)))))

(def (project-search-plan plan)
  (let ((workspace (project-optional-string (.get plan workspace) 'workspace))
        (languages (list->vector
                     (map (lambda (item) (require-symbol-or-string item 'language))
                          (.get plan languages))))
        (documents (list->vector
                     (map (lambda (item) (require-symbol-or-string item 'document))
                          (.get plan documents))))
        (composition (.get plan composition)))
    (unless composition
      (error "search plan requires a composition"))
    (vector workspace
            languages
            documents
            (project-search-composition composition))))
