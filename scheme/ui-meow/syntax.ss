;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in "./objects.ss" make-marlin-ui-keymap)
        (only-in "./funcs.ss" marlin-ui-expand-keypad))
(export define-key-bindings meow-normal meow-edit meow-insert meow-global meow-keypad
        meow-things meow-expand)

(defrules define-key-bindings ()
  ((_ name keypad: aliases group ...)
   (def name (make-marlin-ui-keymap (marlin-ui-expand-keypad (append group ...) aliases))))
  ((_ name group ...)
   (def name (make-marlin-ui-keymap (append group ...)))))

;; Keep the declaration in the same shape as the projected bindings.
(defrules meow-keys ()
  ((_ scope (keys action help) ...)
   '((scope keys action help) ...)))

(defrules meow-normal ()
  ((_ binding ...) (meow-keys normal binding ...)))

(defrules meow-edit ()
  ((_ binding ...) (meow-keys normal binding ...)))

(defrules meow-insert ()
  ((_ binding ...) (meow-keys insert binding ...)))

(defrules meow-global ()
  ((_ binding ...) (meow-keys global binding ...)))

(defrules meow-keypad ()
  ((_ (keys target) ...) '((keys target) ...)))

;; Expose both extents for each declared pair through the same checked keymap.
(defrules meow-things ()
  ((_ (key open close label) ...)
   (append
     (list (list 'normal '(#\, key) '(select-delimited inner open close)
                 (string-append "Inside " label)) ...)
     (list (list 'normal '(#\. key) '(select-delimited bounds open close)
                 (string-append "Around " label)) ...))))

(defrules meow-expand ()
  ((_ tone (key steps at-rest-start?) ...)
   (list (list 'normal '(key) '(expand-selection steps tone at-rest-start?)
               (string-append "Expand to position " (string key))) ...)))
