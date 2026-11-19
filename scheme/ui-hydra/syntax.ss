;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in "./funcs.ss"
                 make-marlin-ui-hydra hydra-color-exit hydra-color-foreign hydra-head
                 hydra-head-with-column))
(export defui-hydra)

(defrules ui-hydra-command ()
  ((_ #f) #f)
  ((_ (name description capability value))
   (list (symbol->string 'name) description 'none 'capability value))
  ((_ name) 'name))

;;; Keyword datums match directly in patterns. The literals list is identifiers.
(defrules ui-hydra-head ()
  ((_ default-exit default-hint key command label column: column-value option ...)
   (hydra-head-with-column
     (ui-hydra-head default-exit default-hint key command label option ...)
     column-value))
  ((_ default-exit default-hint key command color: color)
   (hydra-head 'key (ui-hydra-command command) default-hint (hydra-color-exit 'color)))
  ((_ default-exit default-hint key command exit: exit-value)
   (hydra-head 'key (ui-hydra-command command) default-hint exit-value))
  ((_ default-exit default-hint key command label color: color)
   (hydra-head 'key (ui-hydra-command command) label (hydra-color-exit 'color)))
  ((_ default-exit default-hint key command label exit: exit-value)
   (hydra-head 'key (ui-hydra-command command) label exit-value))
  ((_ default-exit default-hint key command label)
   (hydra-head 'key (ui-hydra-command command) label default-exit))
  ((_ default-exit default-hint key command)
   (hydra-head 'key (ui-hydra-command command) default-hint default-exit)))

(defrules define-hydra ()
  ((_ binding identity-value shortcut color title default-hint hint-mode
      (head-key command-name head-option ...) ...)
   (def binding
     (let (default-exit (hydra-color-exit 'color))
       (make-marlin-ui-hydra 'identity-value title
         (hydra-color-foreign 'color)
         (list (ui-hydra-head default-exit default-hint head-key command-name head-option ...) ...)
         'hint-mode 'shortcut)))))

;;; A declaration supplies the command descriptors and per-head rules.
(defrules defui-hydra ()
  ((_ binding (identity-value color: color key: shortcut hint: #f) title head ...)
   (define-hydra binding identity-value shortcut color title #f none head ...))
  ((_ binding (identity-value color: color key: shortcut) title head ...)
   (define-hydra binding identity-value shortcut color title "" auto head ...))
  ((_ binding (identity-value color: color hint: #f) title head ...)
   (define-hydra binding identity-value #f color title #f none head ...))
  ((_ binding (identity-value color: color) title head ...)
   (define-hydra binding identity-value #f color title "" auto head ...)))
