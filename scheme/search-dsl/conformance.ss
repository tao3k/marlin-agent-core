;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Helm TAB Completion Introspection, ASP Language Conformance Packs,
;;; JSON Schema Projection & Dynamic Registry Extension
;;; (`scheme/search-dsl/conformance.ss`).
(import (only-in :clan/poo/object .get .o .ref seal-object!)
        (only-in :gerbil-ascent/rules/closed-source ascent-closed-dsl-parts)
        "./syntax.ss"
        "./objects.ss"
        "./prefilter.ss"
        "./completions.ss")
(export search-dsl-completions
        json-schema->completion-entries
        json-schema-file->completion-entries
        json-schema-string->completion-entries
        search-dsl-tool-description
        search-dsl-tool-summary
        project-language-conformance-pack
        project-all-conformance-packs
        project-language-json-schema
        extend-search-language-registry)

(def (search-dsl-tool-description (_registry SearchLanguageRegistryValue))
  (.get WorkspaceSearchToolSpecValue summary))

(def (search-dsl-tool-summary (_registry SearchLanguageRegistryValue))
  (.get WorkspaceSearchToolSpecValue summary))

(def (contract-entry->conformance-pack entry)
  (let ((lang-sym (list-ref entry 0))
        (provider-lang (list-ref entry 1))
        (extensions (list-ref entry 2))
        (markers (list-ref entry 3))
        (kind-map (list-ref entry 4))
        (special-rels (list-ref entry 5))
        (probes (list-ref entry 6)))
    (list->hash-table
      (list
        (cons "schemaId" "agent.semantic-protocols.language-conformance-pack")
        (cons "schemaVersion" "1")
        (cons "languageId" (symbol->string lang-sym))
        (cons "providerLanguageId" provider-lang)
        (cons "extensions" extensions)
        (cons "projectMarkers" markers)
        (cons "kindMappings"
              (map (lambda (km)
                     (list->hash-table
                       (list (cons "nativeLanguageKind" (list-ref km 0))
                             (cons "generalKind" (list-ref km 1))
                             (cons "description" (list-ref km 2)))))
                   kind-map))
        (cons "specialRelations" (map symbol->string special-rels))
        (cons "probes"
              (map (lambda (probe)
                     (list->hash-table
                       (list (cons "probeId" (list-ref probe 0))
                             (cons "path" (list-ref probe 1))
                             (cons "source" (list-ref probe 2))
                             (cons "expectValid" (list-ref probe 3))
                             (cons "expectedNodes"
                                   (map (lambda (node)
                                          (list->hash-table
                                            (list (cons "kind" (list-ref node 0))
                                                  (cons "languageKind" (list-ref node 1))
                                                  (cons "name" (list-ref node 2)))))
                                        (list-ref probe 4))))))
                   probes))))))

(def (project-language-conformance-pack lang-key (registry SearchLanguageRegistryValue))
  (let* ((needle (if (symbol? lang-key) (symbol->string lang-key) lang-key))
         (contracts (.get registry language-contracts))
         (matched
           (find (lambda (entry)
                   (or (string=? (symbol->string (list-ref entry 0)) needle)
                       (string=? (list-ref entry 1) needle)))
                 contracts)))
    (if matched
      (contract-entry->conformance-pack matched)
      (error "unknown ASP language contract in SearchLanguageRegistry" lang-key))))

(def (project-all-conformance-packs (registry SearchLanguageRegistryValue))
  (map contract-entry->conformance-pack (.get registry language-contracts)))

(def (unique-strings items)
  (let loop ((rest items) (acc '()))
    (cond
      ((null? rest) (reverse acc))
      ((member (car rest) acc) (loop (cdr rest) acc))
      (else (loop (cdr rest) (cons (car rest) acc))))))

(def (project-language-json-schema lang-key (registry SearchLanguageRegistryValue))
  (let* ((needle (if (symbol? lang-key) (symbol->string lang-key) lang-key))
         (contracts (.get registry language-contracts))
         (matched
           (find (lambda (entry)
                   (or (string=? (symbol->string (list-ref entry 0)) needle)
                       (string=? (list-ref entry 1) needle)))
                 contracts)))
    (unless matched
      (error "unknown ASP language contract in SearchLanguageRegistry" lang-key))
    (let* ((provider-lang (list-ref matched 1))
           (kind-map (list-ref matched 4))
           (native-kinds (unique-strings (map car kind-map)))
           (general-kinds (unique-strings (map cadr kind-map))))
      `((type . "object")
        (additionalProperties . #f)
        (required . ("languageId" "kind" "languageKind" "name" "line"))
        (properties
         . ((languageId   . ((type . "string") (const . ,provider-lang)))
            (kind         . ((type . "string") (enum . ,general-kinds)))
            (languageKind . ((type . "string") (enum . ,native-kinds)))
            (name         . ((type . "string") (minLength . 1)))
            (line         . ((type . "integer") (minimum . 1)))))))))

(def (append-unique-by-key existing incoming key-fn)
  (let ((incoming-keys (map key-fn incoming)))
    (append (filter (lambda (item) (not (member (key-fn item) incoming-keys))) existing)
            incoming)))

(def (extend-search-language-registry base-registry desc-vec)
  (unless (and (vector? desc-vec) (= (vector-length desc-vec) 5))
    (error "extend-search-language-registry requires #(lang provider-lang extensions kind-map special-specs)"))
  (let* ((lang-str (vector-ref desc-vec 0))
         (lang-sym (string->symbol lang-str))
         (provider-lang (vector-ref desc-vec 1))
         (extensions (vector->list (vector-ref desc-vec 2)))
         (kind-map (map vector->list (vector->list (vector-ref desc-vec 3))))
         (special-specs (map vector->list (vector->list (vector-ref desc-vec 4))))
         (existing-rel-syms (map car (.get base-registry relation-meta)))
         (new-special-specs
           (filter (lambda (spec)
                     (not (memq (string->symbol (car spec)) existing-rel-syms)))
                   special-specs))
         (dyn-fragments
           (if (null? new-special-specs)
             '()
             (ascent-closed-dsl-parts
               (list
                 `(ascent-fragment
                    (relation entity
                      ((id string?) (file_id integer?) (lang string?)
                       (kind string?) (language_kind string?)
                       (name string?) (line integer?)))
                    ,@(map (lambda (spec)
                             `(relation ,(string->symbol (car spec))
                                ((id string?) (file_id integer?) (name string?) (line integer?))))
                           new-special-specs)
                    ,@(map (lambda (spec)
                             (let ((rel-sym (string->symbol (car spec)))
                                   (kind-pat (if (and (string? (cadr spec)) (> (string-length (cadr spec)) 0)) (cadr spec) '_))
                                   (lk-pat (if (and (string? (caddr spec)) (> (string-length (caddr spec)) 0)) (caddr spec) '_)))
                               `((,rel-sym id f name line)
                                 <-- (entity id f ,lang-str ,kind-pat ,lk-pat name line))))
                           new-special-specs))))))
         (dyn-meta
           (map (lambda (spec)
                  (let* ((rel-sym (string->symbol (car spec)))
                         (kind-pat (if (string? (cadr spec)) (cadr spec) '_))
                         (lk-pat (if (string? (caddr spec)) (caddr spec) '_))
                         (doc (list-ref spec 3)))
                    (list rel-sym lang-str kind-pat lk-pat doc)))
                new-special-specs))
         (existing-contract (find (lambda (c) (eq? (car c) lang-sym)) (.get base-registry language-contracts)))
         (dyn-contract
           (if existing-contract
             (list lang-sym provider-lang
                   (unique-strings (append (list-ref existing-contract 2) extensions))
                   (list-ref existing-contract 3)
                   (append-unique-by-key (list-ref existing-contract 4) kind-map car)
                   (append-unique-by-key (list-ref existing-contract 5) (map (lambda (s) (string->symbol (car s))) special-specs) (lambda (s) s))
                   (list-ref existing-contract 6))
             (list lang-sym provider-lang extensions '() kind-map
                   (map (lambda (s) (string->symbol (car s))) special-specs) '())))
         (next-langs (if (memq lang-sym (.get base-registry languages))
                       (.get base-registry languages)
                       (append (.get base-registry languages) (list lang-sym))))
         (next-frags (append (.get base-registry ascent-fragments) dyn-fragments))
         (next-meta (append-unique-by-key (.get base-registry relation-meta) dyn-meta car))
         (next-contracts (append-unique-by-key (.get base-registry language-contracts) (list dyn-contract) (lambda (c) (symbol->string (car c))))))
    (seal-object!
      (.o languages: next-langs
          ascent-fragments: next-frags
          relation-meta: next-meta
          language-contracts: next-contracts
          protocol-json-schemas: (.get base-registry protocol-json-schemas)
          syntax-forms: (.get base-registry syntax-forms)))))
