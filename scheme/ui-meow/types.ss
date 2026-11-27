;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc .get)
        (only-in :clan/poo/type Exactly Fun List Record String Type.)
        (only-in :std/list/list length<=n? take))
(export KeyBindings KeypadAliases UiMeowScopeContract marlin-ui-key-spelling)

(def (marlin-ui-key-spelling key)
  (if (char? key) (string key) (symbol->string key)))

(def (bounded-list? value limit)
  (and (length<=n? value limit) (list? value)))

(def (key-sequence? value)
  (and (pair? value)
       (bounded-list? value 4)
       (andmap (lambda (key)
                 (or (symbol? key)
                     (and (char? key) (char<=? #\! key #\~))))
               value)))

(def (delimiter? value)
  (and (char? value) (char<=? #\! value #\~) (not (char=? value #\\))))

(def (key-action? value)
  (or (symbol? value)
      (match value
        (['select-delimited extent open close]
         (and (memq extent '(inner bounds)) (delimiter? open) (delimiter? close)))
        (['expand-selection steps tone at-rest-start?]
         (and (exact-integer? steps) (<= 1 steps 10)
              (memq tone '(normal canvas accent info success warning muted))
              (boolean? at-rest-start?)))
        (['read-selection-character extent direction]
         (and (memq extent '(through before)) (memq direction '(forward backward))))
        (else #f))))

(def (binding? value)
  (match value
    ([scope keys action help]
     (and (memq scope '(global insert normal))
          (key-sequence? keys) (key-action? action) (string? help)
          (<= (string-length help) 96)))
    (else #f)))

(def (keypad-alias? value)
  (match value
    ([keys target] (and (key-sequence? keys) (key-sequence? target)))
    (else #f)))

(def KeyBindings
  (.cc Type. .element?:
    (lambda (bindings)
      (and (bounded-list? bindings 256) (andmap binding? bindings)))))

(def KeypadAliases
  (.cc Type. .element?:
    (lambda (aliases)
      (and (bounded-list? aliases 64) (andmap keypad-alias? aliases)))))

(def KeymapProjection (.cc Type. '.element? vector?))

(def (check-key-overlap keymap)
  (def (prefix? shorter longer)
    (let (count (length shorter))
      (and (<= count (length longer)) (equal? shorter (take longer count)))))
  (def sequences
    (map (lambda (entry) (cons (car entry) (map marlin-ui-key-spelling (cadr entry))))
      (.get keymap bindings)))
  (let loop ((pending sequences))
    (unless (null? pending)
      (let* ((current (car pending)) (scope (car current)) (keys (cdr current)))
        (for-each (lambda (other)
                    (when (and (or (eq? scope 'global) (eq? (car other) 'global)
                                   (eq? scope (car other)))
                               (or (prefix? keys (cdr other)) (prefix? (cdr other) keys)))
                      (error "overlapping UI keybindings" current other)))
          (cdr pending)))
      (loop (cdr pending))))
  keymap)

(def UiMeowScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-meow)]
               bindings: [KeyBindings]
               project: [(Fun KeymapProjection <- KeyBindings)]
               help: [(Fun (List String) <- KeyBindings)])
       identity: 'marlin/ui-meow
       .instantiate: check-key-overlap))
