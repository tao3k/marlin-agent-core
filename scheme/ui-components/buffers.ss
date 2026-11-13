;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Buffer identity and window selection are UI state. Native content stays
;;; with its owner; bounded static content is declared beside its Buffer.
(import (only-in :clan/poo/object .def .get .o seal-object!)
        (only-in :std/list/list find))
(export marlin-ui-make-frame marlin-ui-select-buffer marlin-ui-project-frame
        marlin-ui-buffer-command-entries marlin-ui-buffer-alias-command-entries
        marlin-ui-buffer-static-entries marlin-ui-buffer-identities)

(.def UiBuffer.)
(.def UiWindow.)
(.def UiFrame.)
(.def UiMiniBuffer.)
(.def UiBufferCatalog.
  entries: '((conversation "Conversation buffer" #f #f)
             (messages "*Messages* UI diagnostics" #f #f)
             (help "Keyboard shortcuts"
                   ("*Help*"
                    ("Enter  Send message"
                     "Ctrl-N  Insert newline"
                     "M-x / Ctrl-O  Open commands"
                     "Ctrl-X B  Switch buffer"
                     "Ctrl-R  Search prompts"
                     "Alt-P / Alt-N  Prompt history"
                     "PageUp / PageDown  Scroll current buffer"
                     "Esc / Ctrl-C  Cancel or return"
                     "Ctrl-Q  Quit"))
                   "help")
             (processes "*Processes* managed Runtime processes" #f #f)))
(def marlin-ui-buffer-catalog (seal-object! UiBufferCatalog.))

(def (marlin-ui-buffer-identities)
  (map car (.get marlin-ui-buffer-catalog entries)))

(def (marlin-ui-buffer-static-entries)
  (filter (lambda (entry) (caddr entry))
          (.get marlin-ui-buffer-catalog entries)))

(def (marlin-ui-buffer-alias-command-entries)
  (map (lambda (entry)
         (list (cadddr entry) (cadr entry) 'none 'select-buffer
               (symbol->string (car entry))))
       (filter (lambda (entry) (cadddr entry))
               (.get marlin-ui-buffer-catalog entries))))

(def (marlin-ui-buffer-command-entries)
  (map (lambda (entry)
         (let (identity-value (symbol->string (car entry)))
           (list (string-append "buffer-" identity-value) (cadr entry)
                 'none 'select-buffer identity-value)))
       (.get marlin-ui-buffer-catalog entries)))

(def (marlin-ui-make-frame)
  (let* ((buffer-values (map (lambda (entry)
                               (seal-object! (.o (:: @ UiBuffer.) identity: (car entry))))
                             (.get marlin-ui-buffer-catalog entries)))
         (window (seal-object! (.o (:: @ UiWindow.) identity: 'main
                                    buffer: (car buffer-values))))
         (mini (seal-object! (.o (:: @ UiMiniBuffer.) identity: 'minibuffer))))
    (seal-object! (.o (:: @ UiFrame.) identity: 'main
                      windows: (list window) selected-window: 'main
                      minibuffer: mini buffers: buffer-values))))

(def (marlin-ui-select-buffer frame identity-value)
  (unless (symbol? identity-value) (error "invalid UI buffer identity"))
  (let* ((buffer-values (.get frame buffers))
         (target-buffer (find (lambda (item) (eq? (.get item identity) identity-value)) buffer-values))
         (window-values (.get frame windows))
         (selected (.get frame selected-window)))
    (unless target-buffer (error "unknown UI buffer"))
    (seal-object! (.o (:: @ UiFrame.) identity: (.get frame identity)
                      selected-window: selected minibuffer: (.get frame minibuffer)
                      buffers: buffer-values
                      windows: (map (lambda (window)
                                      (if (eq? (.get window identity) selected)
                                        (seal-object! (.o (:: @ UiWindow.)
                                                          identity: selected buffer: target-buffer))
                                        window))
                                    window-values)))))

(def (marlin-ui-project-frame frame)
  (vector (.get frame identity)
          (list->vector (map (lambda (window)
                               (vector (.get window identity)
                                       (.get (.get window buffer) identity)))
                             (.get frame windows)))
          (.get frame selected-window)
          (.get (.get frame minibuffer) identity)
          (list->vector (map (lambda (buffer) (.get buffer identity))
                             (.get frame buffers)))))
