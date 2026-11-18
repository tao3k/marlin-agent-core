;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Compose one bounded bar from the UI's admitted buffer catalog.
(import (only-in :std/list/list append-map filter)
        (only-in "../ui-components/buffers.ss" marlin-ui-buffer-command-name)
        (only-in "../ui-elements/objects.ss" ui-bar ui-template))
(export marlin-ui-buffer-tabs)

(def builtin-tabs
  '((conversation "Chat" session) (sessions "Sessions" session)
    (messages "Messages" session) (models "Models" workspace)
    (processes "Processes" workspace) (help "Help" workspace)))

(def (tab-label entry active?)
  (ui-template (if active? 'accent 'bar)
    (list (list 'literal (if active? 'canvas 'normal) active?
                (string-append " " (cadr entry) " ")))))

(def (tab-segments entry)
  (def identity (car entry))
  (def name (symbol->string identity))
  (def command (string->symbol (marlin-ui-buffer-command-name identity)))
  (map (lambda (active?)
         (list 'left (if active? 0 3) 0
               (string->symbol
                 (string-append (if active? "binding-is/" "binding-is-not/")
                                "active-buffer/" name))
               (tab-label entry active?) command))
       '(#t #f)))

(def (marlin-ui-buffer-tabs identities (declarations '()))
  (def entries
    (map (lambda (id) (or (assq id declarations) (assq id builtin-tabs)
                         (list id (symbol->string id) 'packages)))
         identities))
  (def (group-segments group)
    (let (members (filter (lambda (entry) (eq? (caddr entry) (car group))) entries))
      (if (null? members) '()
        (cons (list 'left 2 0 #f
                    (ui-template 'bar (list (list 'literal 'muted #f (cadr group)))))
              (append-map tab-segments members)))))
  (ui-bar 'bar 1
    (append-map group-segments '((session " › ") (workspace " │ ") (packages " │ ")))))
