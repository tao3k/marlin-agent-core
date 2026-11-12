;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; UI operations exported by the selected Marlin module.
(import (only-in :clan/poo/object .get .o)
        "./workbench.ss" "./dispatch.ss" "./projection.ss")
(export make-ui-operations)
(def (make-ui-operations ui-components)
  (.o workbench: (make-marlin-ui-workbench ui-components)
      project-workbench: marlin-ui-project-workbench
      focus-event: marlin-ui-focus-event
      previous-focus-event: marlin-ui-previous-focus-event
      make-dispatch: make-ui-event-generic dispatch: ui-event-dispatch
      event-protocol: UiEventProtocol
      components: (.get ui-components components)
      events: (.o base: UiEvent. next-focus: UiNextFocus. previous-focus: UiPreviousFocus. submit: UiSubmit.
                  dismiss: UiDismiss. scroll: UiScroll. browse-command: UiBrowseCommand.)))
