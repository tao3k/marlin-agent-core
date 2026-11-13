;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; UI operations exported by the selected Marlin module.
(import (only-in :clan/poo/object .get .o)
        "./workbench.ss" "./dispatch.ss" "./projection.ss")
(export make-ui-operations)
(def (make-ui-operations ui-components ui-modeline ui-transcript ui-themes ui-layout)
  (.o workbench: (make-marlin-ui-workbench ui-components ui-modeline ui-transcript ui-themes ui-layout)
      profile-workbench: (make-marlin-ui-profile-workbench ui-components ui-modeline ui-transcript ui-themes ui-layout)
      project-workbench: marlin-ui-project-workbench
      focus-event: marlin-ui-focus-event
      previous-focus-event: marlin-ui-previous-focus-event
      make-dispatch: make-ui-event-generic dispatch: ui-event-dispatch
      event-protocol: UiEventProtocol
      components: (.get ui-components components)
      make-frame: (.get ui-components make-frame)
      select-buffer: (.get ui-components select-buffer)
      project-frame: (.get ui-components project-frame)
      keymap-entries: (.get ui-components keymap-entries)
      transient-entries: (.get ui-components transient-entries)
      command-candidates: (.get ui-components command-candidates)
      record-command-history: (.get ui-components record-command-history)
      review-menu: (.get ui-components review-menu)
      events: (.o base: UiEvent. next-focus: UiNextFocus. previous-focus: UiPreviousFocus. submit: UiSubmit.
                  dismiss: UiDismiss. scroll: UiScroll. browse-command: UiBrowseCommand.)))
