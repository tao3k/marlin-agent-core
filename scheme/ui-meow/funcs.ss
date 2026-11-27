;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :std/list/list delete-duplicates/hash drop filter-map find group-n-consecutive take)
        (only-in :clan/poo/type validate)
        (only-in "./types.ss" KeyBindings KeypadAliases marlin-ui-key-spelling)
        (only-in "../ui-elements/funcs.ss" ui-key-hint-table))
(export marlin-ui-expand-keypad marlin-ui-project-keymap marlin-ui-format-keymap)

(def (marlin-ui-expand-keypad declarations shortcuts)
  (def bindings (validate KeyBindings declarations))
  (def aliases (validate KeypadAliases shortcuts))
  ;; Apply the modifier only to the first key. A sequence such as C-h r
  ;; remains a single binding and becomes SPC c h r in Keypad.
  (def (modifier-alias entry)
    (let* ((target (cadr entry))
           (key (marlin-ui-key-spelling (car target)))
           (modifier (cond ((string-prefix? "C-" key) 'c)
                           ((string-prefix? "M-" key) 'm)
                           (else #f))))
      (and modifier (<= (length target) 2)
           (let (keys (cons* 'SPC modifier (string->symbol (substring key 2 (string-length key)))
                         (cdr target)))
             (and (not (assoc keys aliases)) (list keys target))))))
  (let (eligible (filter (lambda (entry) (memq (car entry) '(global normal))) bindings))
    (append bindings
      (map (lambda (alias)
             (let (binding (find (lambda (entry) (equal? (cadr entry) (cadr alias))) eligible))
               (unless binding (error "unknown Keypad target" alias))
               (cons* 'normal (car alias) (cddr binding))))
           (append aliases (filter-map modifier-alias eligible))))))

(def (project-key key)
  (if (char? key) (string->symbol (string key)) key))

(def (keypad-head prefix keys description)
  (let* ((key (car keys)) (spelling (marlin-ui-key-spelling key)))
    (if (null? (cdr keys))
      (list spelling description 'normal)
      (list spelling
            (or (and (equal? prefix '(SPC))
                     (assget key '((b . "Buffers") (c . "Control") (m . "Meta")
                                   (t . "Themes") (w . "Windows") (TAB . "Workspaces"))))
                "More bindings")
            'info))))

(def (keypad-prefix-entries bindings)
  ;; Echo is immediate. Give a continued keystroke precedence over showing help.
  (def describe-delay-ms 500)
  (def keypad (filter (lambda (entry)
                        (and (eq? (car entry) 'normal) (eq? (caadr entry) 'SPC)
                             (pair? (cdadr entry))))
                      bindings))
  (def prefixes
    (delete-duplicates/hash
      (apply append
        (map (lambda (entry)
               (let loop ((prefix '(SPC)) (rest (cdadr entry)))
                 (if (null? rest) '()
                   (cons prefix (loop (append prefix (list (car rest))) (cdr rest))))))
             keypad))))
  (map (lambda (prefix)
         (let* ((count (length prefix))
                (heads (delete-duplicates/hash
                         (filter-map (lambda (entry)
                                       (let (keys (cadr entry))
                                         (and (< count (length keys))
                                              (equal? prefix (take keys count))
                                              (keypad-head prefix (drop keys count) (cadddr entry)))))
                           keypad)
                         key: car))
                (visible (take heads (min 12 (length heads)))))
           (def (heads-with tone)
             (filter (lambda (head) (eq? (caddr head) tone)) visible))
           (vector 'normal (list->vector (map project-key prefix))
                   (vector 'prefix-hint
                           (ui-key-hint-table
                             (list (list "Commands" (heads-with 'normal))
                                   (list "Keymaps" (heads-with 'info))))
                           describe-delay-ms))))
    prefixes))

(def (marlin-ui-project-keymap bindings)
  (list->vector
    (append (map (lambda (entry)
           (let (keys (map project-key (cadr entry)))
             (let (action (caddr entry))
               (vector (car entry) (list->vector keys)
                       (if (pair? action) (list->vector action) action)))))
      bindings)
      (keypad-prefix-entries bindings))))

(def (marlin-ui-format-keymap bindings)
  (map (lambda (group) (string-join group "  ·  "))
       (group-n-consecutive 4
         (map (lambda (entry)
                (string-append (if (eq? (car entry) 'normal) "[normal] " "")
                               (string-join (map marlin-ui-key-spelling (cadr entry)) " ")
                               "  " (cadddr entry)))
              bindings))))
