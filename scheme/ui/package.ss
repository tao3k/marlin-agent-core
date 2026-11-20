;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; A package extends the selected scopes using ordinary POO values. Each
;;; session retains its catalog and activation state.
(import (only-in :clan/poo/object .def .get .o)
        (only-in :tao3k/yuanxu/prelude yuanxu-module-declaration)
        (only-in "../scheme-package-hooks/config.ss" SchemePackageHooksScope)
        (only-in "../scheme-package-hooks/funcs.ss" run-ui-package-hooks)
        (only-in :std/list/list duplicates find)
        (only-in "../ui-components/bounds.ss" marlin-ui-bounded-list?)
        (only-in "../ui-components/commands.ss" marlin-ui-command-entries marlin-ui-command-entries?)
        (only-in "../ui-components/buffers.ss" marlin-ui-buffer-catalog
                 marlin-ui-buffer-identities marlin-ui-buffer-command-entries
                 marlin-ui-buffer-alias-command-entries marlin-ui-make-frame)
        (only-in "../ui-components/components.ss" marlin-ui-static-buffer-nodes)
        (only-in "../ui-meow/objects.ss" make-marlin-ui-keymap)
        (only-in "../ui-components/config.ss" UiComponentsScope)
        (only-in "../ui-modeline/config.ss" UiModelineScope)
        (only-in "../ui-transcript/config.ss" UiTranscriptScope)
        (only-in "../ui-themes/config.ss" UiThemesScope)
        (only-in "../ui-layout/config.ss" UiLayoutScope)
        (only-in "./config.ss" make-ui-scope)
        (only-in "./package-types.ss" UiPackageDeclaration))
(export UiPackage. make-ui-package marlin-ui-package-catalog
        marlin-ui-package-preview marlin-ui-package-publication marlin-ui-package-check-commands)

;;; Resolve data separately from the implementation. Buffer metadata uses the
;;; same command projection as the full publication, without constructing bodies.
(def (marlin-ui-package-catalog directory-value module-spec export-name)
  (let* ((declaration
           (yuanxu-module-declaration module-spec export-name UiPackageDeclaration
             directory: directory-value))
         (buffers (.o entries: (map (lambda (entry)
                                      (list (car entry) (cadr entry) #f (caddr entry)))
                                    (.get declaration buffers))))
         (command-entries (append (.get declaration commands)
                                  (marlin-ui-buffer-alias-command-entries buffers)
                                  (marlin-ui-buffer-command-entries buffers))))
    (when (ormap (lambda (entry) (eq? (cadddr entry) 'deferred)) command-entries)
      (error "a UI package must advertise real command descriptors"))
    (.o directory: directory-value module: (.get declaration module) export: (.get declaration export)
        commands: (marlin-ui-command-entries (.o entries: command-entries))
        keymap: (.get declaration keymap))))

(def (marlin-ui-package-preview catalog)
  (make-ui-package
    (.o (:: @ UiPackage.) keymap: (.get catalog keymap)
        commands: (map (lambda (entry) (list (car entry) (cadr entry) (caddr entry) 'deferred #f))
                       (.get catalog commands)))))

(def (marlin-ui-package-publication catalog event command-name)
  (unless (assoc command-name (.get catalog commands))
    (error "command is not declared by the UI package" command-name))
  (run-ui-package-hooks SchemePackageHooksScope
    (.o directory: (.get catalog directory) module: (.get catalog module)
        export: (.get catalog export) trigger: event command: command-name)))

(def (marlin-ui-package-check-commands catalog entries)
  (for-each (lambda (declared)
              (unless (equal? (assoc (car declared) entries) declared)
                (error "published UI command differs from its declaration" (car declared))))
            (.get catalog commands)))

(.def UiPackage.
  buffers: '()
  commands: '()
  keymap: '()
  layout: UiLayoutScope
  method-factories: #())

(def (make-ui-package package-value)
  (let* ((components UiComponentsScope)
         (buffers (.get package-value buffers))
         (keys (.get package-value keymap))
         (commands (.get package-value commands))
         (_admission
           (unless (and (marlin-ui-bounded-list? buffers 8)
                        (marlin-ui-bounded-list? keys 16)
                        (marlin-ui-command-entries? commands)
                        (andmap (lambda (entry)
                                  (and (marlin-ui-bounded-list? entry 4)
                                       (= (length entry) 4) (symbol? (car entry))
                                       (string? (cadr entry))
                                       (marlin-ui-bounded-list? (caddr entry) 2)
                                       (= (length (caddr entry)) 2)
                                       (string? (car (caddr entry)))
                                       (vector? (cadr (caddr entry)))
                                       (or (not (cadddr entry)) (string? (cadddr entry)))))
                                buffers))
             (error "invalid UI package contribution")))
         (catalog (.o entries: (append (.get marlin-ui-buffer-catalog entries) buffers)))
         (keymap (make-marlin-ui-keymap (append (.get components key-bindings) keys)))
         (projected-keymap ((.get keymap project) (.get keymap bindings)))
         (ids (map car (.get catalog entries))))
    (unless (null? (duplicates ids eq?)) (error "duplicate package buffer"))
    (let* ((base (.get components command-catalog))
           (catalog-entries (append
                      (filter (lambda (entry)
                                (not (eq? (cadddr entry) 'select-buffer)))
                              (.get base entries))
                      (marlin-ui-buffer-alias-command-entries catalog)
                      (marlin-ui-buffer-command-entries catalog)))
           (selected-components
             (.o (:: @ components)
                 command-catalog: (.o entries: catalog-entries)
                 command-extensions: (lambda ()
                                       (append ((.get components command-extensions)) commands))
                 buffer-identities: (lambda () (marlin-ui-buffer-identities catalog))
                 static-buffer-nodes: (lambda () (marlin-ui-static-buffer-nodes catalog))
                 make-frame: (lambda () (marlin-ui-make-frame catalog))
                 key-bindings: (.get keymap bindings)
                 keymap-entries: (lambda () projected-keymap))))
      (make-ui-scope selected-components UiModelineScope UiTranscriptScope
                     UiThemesScope (.get package-value layout)
                     method-factories: (.get package-value method-factories)))))
