;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Scheme owns presentation choices; the native host clips them to cells.
(import (only-in :clan/poo/object .def .o seal-object!))
(export marlin-ui-layout-policy)

(def MessageFrameValue
  (seal-object! (.o min-transcript-rows: 2 fixed-editor-rows: 0
                    footer-rows: 1 extensions: #t bordered: #t prompt: " Message ")))
(def CommandFrameValue
  (seal-object! (.o min-transcript-rows: 0 fixed-editor-rows: 1
                    footer-rows: 0 extensions: #f bordered: #f prompt: " M-x ")))
(.def UiLayoutBase.
  message-frame: MessageFrameValue
  command-frame: CommandFrameValue
  max-chatbox-rows: 10
  max-command-rows: 32
  thinking-collapsed: #t
  user-prefix: "› "
  assistant-role: ""
  thinking-collapsed-role: "▸ Thought"
  thinking-expanded-role: "▾ Thought")
(def marlin-ui-layout-policy
  (seal-object! UiLayoutBase.))
