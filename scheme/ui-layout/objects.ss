;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later


(import (only-in :clan/poo/object .def .o seal-object!))
(export UiLayoutBase. frame-regions)

(def MessageFrameValue
  (seal-object! (.o min-transcript-rows: 2 fixed-editor-rows: 0
                    footer-rows: 1 extensions: #t prompt: " Message ")))
(def CommandFrameValue
  (seal-object! (.o min-transcript-rows: 4 fixed-editor-rows: 1
                    footer-rows: 0 extensions: #f prompt: " M-x ")))

;;; Named region bindings let the layout express priorities without host-side
;;; knowledge of any particular component or its neighbours.
;; A host role is not a widget name. The extent expression and region node
;; both reference the identity selected by this layout.
(def frame-regions
  '((transcript . transcript) (extensions . extensions) (modeline . modeline)
    (editor . composer-window) (candidates . candidates) (minibuffer . minibuffer)
    (minibuffer-hover . minibuffer-envelope)))

(.def UiLayoutBase.
  message-frame: MessageFrameValue
  command-frame: CommandFrameValue
  region-bindings: (list->vector (map (lambda (binding) (vector (car binding) (cdr binding))) frame-regions))
  max-chatbox-rows: 10
  max-command-rows: 10)
