;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure functional Emacs state transitions (`scheme/ui-emacs`):
;;; Bookmarks (C-x r m / C-x r b), Kill Ring (kill-ring-save / yank / yank-pop),
;;; Better-Jumper navigation stack (C-o / C-i), and native C-ABI vector projection
;;; for Consult virtual sources.
(import (only-in :clan/poo/object .cc .get .o)
        (only-in :std/list/list filter find take)
        "./objects.ss")
(export marlin-emacs-make-session
        marlin-emacs-bookmark-set
        marlin-emacs-bookmark-jump
        marlin-emacs-bookmark-delete
        marlin-emacs-bookmark-list
        marlin-emacs-kill-push
        marlin-emacs-yank-current
        marlin-emacs-yank-pop
        marlin-emacs-jump-push
        marlin-emacs-jump-back
        marlin-emacs-jump-forward
        marlin-emacs-push-mark
        marlin-emacs-pop-global-mark
        marlin-emacs-add-hook
        marlin-emacs-remove-hook
        marlin-emacs-run-hooks
        marlin-emacs-project-consult-sources)

(def (marlin-emacs-make-session)
  DefaultEmacsSession)

;; --- Bookmarks (Emacs bookmark.el) ---

(def (marlin-emacs-bookmark-set session name path line (context "") (project-root ""))
  (let* ((clean-name (string-trim-both name))
         (existing (filter (lambda (b) (not (string=? (.get b name) clean-name)))
                           (.get session bookmarks)))
         (entry (.o name: clean-name
                    path: path
                    line: (if (< line 1) 1 line)
                    context: context
                    project-root: project-root)))
    (.cc session bookmarks: (cons entry existing))))

(def (marlin-emacs-bookmark-jump session name)
  (find (lambda (b) (string=? (.get b name) name))
        (.get session bookmarks)))

(def (marlin-emacs-bookmark-delete session name)
  (.cc session
       bookmarks: (filter (lambda (b) (not (string=? (.get b name) name)))
                          (.get session bookmarks))))

(def (marlin-emacs-bookmark-list session)
  (.get session bookmarks))

;; --- Kill Ring (Emacs simple.el + consult-yank-pop) ---

(def (marlin-emacs-kill-push session text)
  (let* ((kr (.get session kill-ring))
         (entries (.get kr entries))
         (cap (.get kr max-size)))
    (if (string=? text "")
      session
      (let* ((deduped (filter (lambda (s) (not (string=? s text))) entries))
             (next-all (cons text deduped))
             (bounded (if (> (length next-all) cap) (take next-all cap) next-all)))
        (.cc session
             kill-ring: (.cc kr entries: bounded yank-index: 0))))))

(def (marlin-emacs-yank-current session)
  (let* ((kr (.get session kill-ring))
         (entries (.get kr entries))
         (idx (.get kr yank-index)))
    (if (null? entries)
      #f
      (list-ref entries (modulo idx (length entries))))))

(def (marlin-emacs-yank-pop session (step 1))
  (let* ((kr (.get session kill-ring))
         (entries (.get kr entries)))
    (if (null? entries)
      (values session #f)
      (let* ((next-idx (modulo (+ (.get kr yank-index) step) (length entries)))
             (next-kr (.cc kr yank-index: next-idx))
             (text (list-ref entries next-idx)))
        (values (.cc session kill-ring: next-kr) text)))))

;; --- Better-Jumper Navigation Stack (Doom Emacs C-o / C-i) ---

(def (marlin-emacs-jump-push session buffer-id path line (label ""))
  (let* ((jl (.get session jump-list))
         (entries (.get jl entries))
         (cursor (.get jl cursor))
         (cap (.get jl max-size))
         ;; Truncate forward history when jumping from mid-stack (authentic better-jumper).
         (prefix (if (and (>= cursor 0) (< (+ cursor 1) (length entries)))
                   (take entries (+ cursor 1))
                   entries))
         (entry (.o buffer-id: buffer-id
                    path: path
                    line: (if (< line 1) 1 line)
                    label: label))
         (same-as-top? (and (pair? prefix)
                            (let ((top (last-elem prefix)))
                              (and (string=? (.get top path) path)
                                   (= (.get top line) (.get entry line)))))))
    (if same-as-top?
      session
      (let* ((appended (append prefix (list entry)))
             (bounded (if (> (length appended) cap)
                        (drop-first appended (- (length appended) cap))
                        appended)))
        (.cc session
             jump-list: (.cc jl
                             entries: bounded
                             cursor: (- (length bounded) 1)))))))

(def (marlin-emacs-jump-back session)
  (let* ((jl (.get session jump-list))
         (entries (.get jl entries))
         (cursor (.get jl cursor)))
    (if (<= cursor 0)
      (values session #f)
      (let* ((next-cur (- cursor 1))
             (target (list-ref entries next-cur)))
        (values (.cc session jump-list: (.cc jl cursor: next-cur)) target)))))

(def (marlin-emacs-jump-forward session)
  (let* ((jl (.get session jump-list))
         (entries (.get jl entries))
         (cursor (.get jl cursor)))
    (if (or (< cursor 0) (>= (+ cursor 1) (length entries)))
      (values session #f)
      (let* ((next-cur (+ cursor 1))
             (target (list-ref entries next-cur)))
        (values (.cc session jump-list: (.cc jl cursor: next-cur)) target)))))

;; --- Global Mark Ring (Emacs simple.el `push-mark` / `pop-global-mark`) ---

(def (marlin-emacs-push-mark session buffer-id path line)
  (let* ((ring (.get session global-mark-ring))
         (norm-line (if (< line 1) 1 line))
         (deduped (filter (lambda (m)
                            (not (and (string=? (.get m path) path)
                                      (= (.get m line) norm-line))))
                          ring))
         (entry (.o buffer-id: buffer-id path: path line: norm-line))
         (next (cons entry deduped)))
    (.cc session global-mark-ring: (if (> (length next) 32) (take next 32) next))))

(def (marlin-emacs-pop-global-mark session)
  (let ((ring (.get session global-mark-ring)))
    (if (null? ring)
      (values session #f)
      (let ((head (car ring))
            (rest (cdr ring)))
        (values (.cc session global-mark-ring: (append rest (list head))) head)))))

;; --- Depth-Ordered Emacs Hook Pipeline (`add-hook` / `remove-hook` / `run-hooks`) ---

(def (insert-hook-by-depth entry sorted)
  (cond
    ((null? sorted) (list entry))
    ((< (.get entry depth) (.get (car sorted) depth))
     (cons entry sorted))
    (else (cons (car sorted) (insert-hook-by-depth entry (cdr sorted))))))

(def (marlin-emacs-add-hook session hook-name fn-id (depth 0))
  (let* ((existing (filter (lambda (h)
                             (not (and (eq? (.get h hook-name) hook-name)
                                       (eq? (.get h fn-id) fn-id))))
                           (.get session hooks)))
         (entry (.o hook-name: hook-name fn-id: fn-id depth: depth)))
    (.cc session hooks: (insert-hook-by-depth entry existing))))

(def (marlin-emacs-remove-hook session hook-name fn-id)
  (.cc session
       hooks: (filter (lambda (h)
                        (not (and (eq? (.get h hook-name) hook-name)
                                  (eq? (.get h fn-id) fn-id))))
                      (.get session hooks))))

(def (marlin-emacs-run-hooks session hook-name)
  (map (lambda (h) (.get h fn-id))
       (filter (lambda (h) (eq? (.get h hook-name) hook-name))
               (.get session hooks))))

;; Native C-ABI FFI vector projection for Consult virtual sources (zero text serialization).
(def (marlin-emacs-project-consult-sources _input)
  (list->vector
    (map (lambda (s)
           (vector (symbol->string (.get s id))
                   (.get s label)
                   (.get s narrow-key)
                   (symbol->string (.get s engine))
                   (.get s priority)
                   (.get s async?)))
         DefaultConsultSources)))

;; Local pure helpers (zero external string-trim dependency).
(def (string-trim-both s)
  (let* ((len (string-length s))
         (start (let loop ((i 0))
                  (if (and (< i len) (char-whitespace? (string-ref s i)))
                    (loop (+ i 1))
                    i)))
         (end (let loop ((i len))
                (if (and (> i start) (char-whitespace? (string-ref s (- i 1))))
                  (loop (- i 1))
                  i))))
    (substring s start end)))

(def (last-elem lst)
  (if (null? (cdr lst)) (car lst) (last-elem (cdr lst))))

(def (drop-first lst n)
  (if (or (<= n 0) (null? lst)) lst (drop-first (cdr lst) (- n 1))))
