;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .o)
        "./components.ss" "./commands.ss")
(export UiComponentsOperations)

(def UiComponentsOperations
  (.o identity: 'marlin/ui-components
      components: (.o base: UiComponent. workbench: UiWorkbench.
                      transcript: UiTranscript. composer: UiComposer.
                      commands: UiCommands. status: UiStatus.
                      status-item: UiStatusItem. extension: UiExtension.
                      guide: UiGuide. text: UiText. field: UiField.)
      contribution-node: marlin-ui-contribution-node
      extension-node: marlin-ui-extension-node
      guide-node: marlin-ui-guide-node
      command-catalog: marlin-ui-command-catalog
      command-entries: marlin-ui-command-entries
      browse-command: marlin-ui-browse-command))
