;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .get .o)
        (only-in :clan/poo/mop .new/checked)
        (only-in "./types.ss" UiComponentsScopeContract)
        (only-in "../ui-meow/key-bindings.ss" UiMeowScope)
        (only-in "../ui-meow/objects.ss" make-marlin-ui-keymap)
        "./components.ss" "./commands.ss" "./command-catalog.ss" "./buffers.ss"
        "./review.ss")
(export make-ui-components-operations)

(def (make-ui-components-operations command-extensions-value transient-entries-value hydra-keys)
  (def keymap (make-marlin-ui-keymap (append (.get UiMeowScope bindings) hydra-keys)))
  (def keymap-entries ((.get keymap project) (.get keymap bindings)))
  (.new/checked UiComponentsScopeContract
    identity: 'marlin/ui-components
    components: (.o base: UiComponent. workbench: UiWorkbench.
                    transcript: UiTranscript. composer: UiComposer.
                    commands: UiCommands. status: UiStatus.
                    status-item: UiStatusItem. extension: UiExtension.
                    text: UiText. field: UiField.)
    contribution-node: marlin-ui-contribution-node
    modeline-node: marlin-ui-modeline-node
    extension-node: marlin-ui-extension-node
    static-buffer-nodes: marlin-ui-static-buffer-nodes
    buffer-identities: marlin-ui-buffer-identities
    command-catalog: marlin-ui-command-catalog
    command-entries: marlin-ui-command-entries
    command-catalog-with-profiles: marlin-ui-command-catalog-with-profiles
    command-catalog-with-extensions: marlin-ui-command-catalog-with-extensions
    command-extensions: command-extensions-value
    transient-entries: transient-entries-value
    command-candidates: marlin-ui-command-candidates
    record-command-history: marlin-ui-record-command-history
    browse-command: marlin-ui-browse-command
    review-menu: marlin-ui-review-menu
    make-frame: marlin-ui-make-frame
    select-buffer: marlin-ui-select-buffer
    project-frame: marlin-ui-project-frame
    key-bindings: (.get keymap bindings)
    keymap-entries: (lambda () keymap-entries)))
