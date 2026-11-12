;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; UI operations exported by the selected Marlin module.
(import (only-in :clan/poo/object .o)
        "./workbench.ss" "./dispatch.ss" "./projection.ss")
(export UiOperations)
(def UiOperations
  (.o workbench: marlin-ui-workbench
      project-workbench: marlin-ui-project-workbench
      focus-event: marlin-ui-focus-event
      previous-focus-event: marlin-ui-previous-focus-event
      make-dispatch: make-ui-event-generic dispatch: ui-event-dispatch
      event-protocol: UiEventProtocol
      components: (.o base: UiComponent. workbench: UiWorkbench. transcript: UiTranscript.
                      composer: UiComposer. commands: UiCommands. status: UiStatus.
                      status-item: UiStatusItem. extension: UiExtension.)
      events: (.o base: UiEvent. next-focus: UiNextFocus. previous-focus: UiPreviousFocus. submit: UiSubmit.
                  dismiss: UiDismiss. scroll: UiScroll. browse-command: UiBrowseCommand.)))
