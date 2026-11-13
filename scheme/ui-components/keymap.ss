;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Emacs-style key sequences are declared once in Scheme and copied into a
;;; bounded native lookup table at UI open. Keystrokes never enter the VM loop.
(import (only-in :clan/poo/object .def .get seal-object!))
(export marlin-ui-keymap-entries marlin-ui-keymap-help)

(.def UiKeymap.
  entries: '(((M-x) open-commands "Open commands")
             ((C-o) open-commands "Open commands")
             ((C-g) cancel "Cancel or return")
             ((C-r) history-search "Search prompts")
             ((M-p) previous-prompt "Previous prompt")
             ((M-n) next-prompt "Next prompt")
             ((C-n) insert-newline "Insert newline")
             ((C-b) backward-char "Move one character left")
             ((C-f) forward-char "Move one character right")
             ((C-d) delete-char "Delete next character")
             ((C-a) line-start "Start of line")
             ((C-e) line-end "End of line")
             ((C-x b) switch-buffer "Switch buffer")))

(def marlin-ui-keymap (seal-object! UiKeymap.))

(def (marlin-ui-keymap-entries)
  (list->vector
    (map (lambda (entry) (vector (list->vector (car entry)) (cadr entry)))
         (.get marlin-ui-keymap entries))))

(def (marlin-ui-keymap-help)
  (map (lambda (entry)
         (string-append (string-join (map symbol->string (car entry)) " ")
                        "  " (caddr entry)))
       (.get marlin-ui-keymap entries)))
