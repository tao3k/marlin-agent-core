;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Emacs-style key sequences are declared once in Scheme and copied into a
;;; bounded native lookup table at UI open. Keystrokes never enter the VM loop.
(import (only-in :clan/poo/object .def .get seal-object!))
(export marlin-ui-keymap-entries)

(.def UiKeymap.
  entries: '(((M-x) open-commands)
             ((C-o) open-commands)
             ((C-g) cancel)
             ((C-r) history-search)
             ((M-p) previous-prompt)
             ((M-n) next-prompt)
             ((C-n) insert-newline)
             ((C-a) line-start)
             ((C-e) line-end)
             ((C-x b) switch-buffer)))

(def marlin-ui-keymap (seal-object! UiKeymap.))

(def (marlin-ui-keymap-entries)
  (list->vector
    (map (lambda (entry) (vector (list->vector (car entry)) (cadr entry)))
         (.get marlin-ui-keymap entries))))
