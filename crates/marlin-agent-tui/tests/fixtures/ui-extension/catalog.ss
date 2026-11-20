;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .get .o)
        (only-in :tao3k/yuanxu/prelude define-yuanxu-module-declaration)
        (only-in :marlin-agent-core/ui-meow/interface define-key-bindings meow-global
                 meow-keypad meow-things))
(export test-catalog)

(define-key-bindings NotesKeymap
  keypad: (meow-keypad ((SPC z) (M-z)))
  (meow-global ((M-z) open-notes "Package notes"))
  (meow-things (z #\| #\| "pipes")))

(define-yuanxu-module-declaration test-catalog
  (.o module: ':marlin-tui-ui-test/fixture export: 'test-publication
      buffers: '((notes "Package notes" #f))
      commands: '(("open-notes" "Open package notes" none select-buffer "notes"))
      keymap: (.get NotesKeymap bindings)))
