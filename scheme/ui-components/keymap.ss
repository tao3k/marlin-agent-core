;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Emacs-style key sequences are declared once in Scheme and copied into a
;;; bounded native lookup table at UI open. Keystrokes never enter the VM loop.
(import (only-in :clan/poo/object .def .get seal-object!)
        (only-in :std/list/list group-n-consecutive))
(export marlin-ui-keymap-entries marlin-ui-keymap-help)

(.def UiKeymap.
  entries: '((global (M-x) open-commands "Open commands")
             (global (C-o) open-commands "Open commands")
             (global (C-g) cancel "Cancel or return")
             (global (C-r) history-search "Search prompts")
             (global (M-p) previous-prompt "Previous prompt")
             (global (M-n) next-prompt "Next prompt")
             (global (C-n) insert-newline "Insert newline")
             (global (C-b) backward-char "Move one character left")
             (global (C-f) forward-char "Move one character right")
             (global (C-d) delete-char "Delete next character")
             (global (C-a) line-start "Start of line")
             (global (C-e) line-end "End of line")
             (global (C-x b) switch-buffer "Switch buffer")
             (insert (ESC) normal-mode "Enter normal mode")
             (normal (ESC) cancel "Cancel or return")
             (normal (i) insert-mode "Insert before cursor")
             (normal (a) append-insert "Insert after cursor")
             (normal (h) backward-char "Move left")
             (normal (j) next-line "Move down")
             (normal (k) previous-line "Move up")
             (normal (l) forward-char "Move right")
             (normal (w) next-word "Next word")
             (normal (b) previous-word "Previous word")
             (normal (Digit0) line-start "Start of line")
             (normal (Dollar) line-end "End of line")
             (normal (x) delete-char "Delete character")
             (normal (d d) delete-line "Delete line")
             (normal (u) undo "Undo edit")
             (normal (RET) next-line "Move down")
             (normal (BACKSPACE) backward-char "Move left")
             (normal (DELETE) delete-char "Delete character")
             (normal (SPC x) open-commands "Open M-x")
             (normal (SPC b b) switch-buffer "Switch buffer")
             (normal (SPC t t) switch-themes "Switch theme")
             (normal (SPC h) help "Open help")))

(def marlin-ui-keymap (seal-object! UiKeymap.))

(def (marlin-ui-keymap-entries)
  (list->vector
    (map (lambda (entry) (vector (car entry) (list->vector (cadr entry)) (caddr entry)))
         (.get marlin-ui-keymap entries))))

(def (marlin-ui-keymap-help)
  (map (lambda (group) (string-join group "  ·  "))
       (group-n-consecutive
         4
         (map (lambda (entry)
                (string-append (if (eq? (car entry) 'normal) "[normal] " "")
                               (string-join (map (lambda (key)
                                                   (case key ((Digit0) "0") ((Dollar) "$")
                                                     (else (symbol->string key))))
                                                 (cadr entry)) " ")
                               "  " (cadddr entry)))
              (.get marlin-ui-keymap entries)))))
