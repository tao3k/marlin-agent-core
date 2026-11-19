;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; A package extends the selected scopes using ordinary POO values. There is
;;; no process-global registry: each publication retains its own catalogs.
(import (only-in :tao3k/yuanxu/prelude yuanxu-module-export)
        (only-in :clan/poo/object .def .get .o)
        (only-in :std/list/list duplicates)
        (only-in "../ui-components/bounds.ss" marlin-ui-bounded-list?)
        (only-in "../ui-components/commands.ss" marlin-ui-command-entries?)
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
        (only-in "./config.ss" make-ui-scope))
(export UiPackage. make-ui-package marlin-ui-load-publication)

;;; Explicit startup operation on the existing VM and package environment.
;;; Use Gerbil's import/export semantics, including renamed/re-exported values;
;;; neither Rust nor this module maintains an alternate module registry/path.
(def (marlin-ui-load-publication input)
  (unless (and (vector? input) (= (vector-length input) 3)
               (string? (vector-ref input 0))
               (< 0 (string-length (vector-ref input 0)) 4097)
               (symbol? (vector-ref input 1)) (symbol? (vector-ref input 2)))
    (error "invalid UI package publication request"))
  (parameterize ((current-directory (path-expand (vector-ref input 0))))
    (yuanxu-module-export (vector-ref input 1) (vector-ref input 2))))

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
