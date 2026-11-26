;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure functional operations for Marlin's Doom-inspired Workspace isolation
;;; (persp-mode) and declarative Popup Window Rule manager (set-popup-rule!).
(import (only-in :std/list/list delete-duplicates/hash filter)
        (only-in :clan/poo/object .get .o seal-object!)
        (only-in "./objects.ss" PopupRule. Workspace. DefaultPopupRules))
(export marlin-ws-make-state
        marlin-ws-make-popup-rule
        marlin-ws-add-popup-rule
        marlin-ws-match-popup-rule
        marlin-ws-create
        marlin-ws-switch
        marlin-ws-kill
        marlin-ws-open-buffer
        marlin-ws-dismiss-popup
        marlin-ws-toggle-popup
        marlin-ws-raise-popup
        marlin-ws-buffers
        marlin-ws-all-buffers
        marlin-ws-isolate-candidates
        marlin-ws-project-state
        marlin-ws-project-popup-rules)

(def (clamp-ratio size)
  (cond
    ((not (number? size)) 0.35)
    ((< size 0.15) 0.15)
    ((> size 0.70) 0.70)
    (else size)))

(def (valid-side side)
  (if (memq side '(bottom right left top)) side 'bottom))

(def (valid-quit quit)
  (if (memq quit '(current other never)) quit 'current))

(def (valid-ttl ttl)
  (if (memq ttl '(ephemeral session persistent)) ttl 'ephemeral))

(def (append-unique lst item)
  (if (member item lst)
      lst
      (append lst (list item))))

(def (make-workspace id name project-root active-buffer buffers
                     popup-buffer popup-rule popup-visible? last-popup-buffer layout-mode)
  (seal-object!
    (.o (:: @ Workspace.)
        (id := id)
        (name := name)
        (project-root := project-root)
        (active-buffer := active-buffer)
        (buffers := buffers)
        (popup-buffer := popup-buffer)
        (popup-rule := popup-rule)
        (popup-visible? := popup-visible?)
        (last-popup-buffer := last-popup-buffer)
        (layout-mode := layout-mode))))

(def (make-state active-id workspaces popup-rules)
  (seal-object!
    (.o (active-id := active-id)
        (workspaces := workspaces)
        (popup-rules := popup-rules))))

(def (find-first pred lst)
  (let loop ((rest lst))
    (cond
      ((null? rest) #f)
      ((pred (car rest)) (car rest))
      (else (loop (cdr rest))))))

(def (str-prefix? prefix text)
  (let ((plen (string-length prefix))
        (tlen (string-length text)))
    (and (<= plen tlen)
         (string=? prefix (substring text 0 plen)))))

(def (find-workspace workspaces id)
  (find-first (lambda (ws) (equal? (.get ws id) id)) workspaces))

(def (replace-workspace workspaces updated)
  (let ((target-id (.get updated id)))
    (map (lambda (ws)
           (if (equal? (.get ws id) target-id) updated ws))
         workspaces)))

(def (active-workspace state)
  (or (find-workspace (.get state workspaces) (.get state active-id))
      (car (.get state workspaces))))

;;; Construct a validated PopupRule object (Doom set-popup-rule! value).
(def (marlin-ws-make-popup-rule matcher side size select? quit ttl modeline?)
  (seal-object!
    (.o (:: @ PopupRule.)
        (matcher := matcher)
        (side := (valid-side side))
        (size := (clamp-ratio size))
        (slot := 0)
        (select? := (and select? #t))
        (quit := (valid-quit quit))
        (ttl := (valid-ttl ttl))
        (modeline? := (and modeline? #t)))))

;;; Construct the initial WorkspaceManager state with the primary "main"
;;; perspective and Doom-curated DefaultPopupRules.
(def (marlin-ws-make-state)
  (make-state
    "main"
    (list (make-workspace "main" "main" "" "*Dashboard*"
                          '("*Dashboard*" "*conversation*")
                          #f #f #f #f 'single))
    DefaultPopupRules))

;;; Prepend/override a declarative popup rule (most recent rule wins, matching
;;; Doom Emacs set-popup-rule! precedence).
(def (marlin-ws-add-popup-rule state rule)
  (let* ((matcher (.get rule matcher))
         (filtered (filter (lambda (r) (not (equal? (.get r matcher) matcher)))
                           (.get state popup-rules))))
    (make-state (.get state active-id)
                (.get state workspaces)
                (cons rule filtered))))

(def (rule-matches-buffer? rule buffer-name)
  (let ((m (.get rule matcher)))
    (and (string? m)
         (> (string-length m) 0)
         (or (string=? m buffer-name)
             (and (str-prefix? m buffer-name)
                  (char=? (string-ref m (- (string-length m) 1)) #\*))
             (str-prefix? m buffer-name)))))

;;; Resolve the governing PopupRule for buffer-name, or #f when it belongs in
;;; the primary editing window.
(def (marlin-ws-match-popup-rule state buffer-name)
  (find-first (lambda (r) (rule-matches-buffer? r buffer-name))
              (.get state popup-rules)))

;;; Create and focus a fresh isolated perspective workspace (optionally bound
;;; to a project root, matching Doom :ui workspaces + projectile integration).
(def (marlin-ws-create state id name project-root)
  (let ((existing (find-workspace (.get state workspaces) id)))
    (if existing
        (make-state id (.get state workspaces) (.get state popup-rules))
        (let* ((initial-buf "*conversation*")
               (ws (make-workspace id name project-root initial-buf
                                   (list "*Dashboard*" initial-buf)
                                   #f #f #f #f 'single)))
          (make-state id
                      (append (.get state workspaces) (list ws))
                      (.get state popup-rules))))))

;;; Switch active workspace, preserving each workspace's exact buffer list,
;;; main window focus, and popup drawer state.
(def (marlin-ws-switch state id)
  (if (find-workspace (.get state workspaces) id)
      (make-state id (.get state workspaces) (.get state popup-rules))
      state))

;;; Kill a workspace unless it is the last remaining perspective.
(def (marlin-ws-kill state id)
  (let ((wss (.get state workspaces)))
    (if (or (<= (length wss) 1) (not (find-workspace wss id)))
        state
        (let* ((remaining (filter (lambda (ws) (not (equal? (.get ws id) id))) wss))
               (next-active (if (equal? (.get state active-id) id)
                                (.get (car remaining) id)
                                (.get state active-id))))
          (make-state next-active remaining (.get state popup-rules))))))

;;; Open a buffer inside the active workspace:
;;; - If governed by a PopupRule, mount it into the workspace's isolated popup
;;;   drawer WITHOUT disturbing active-buffer or main window tabs.
;;; - Otherwise focus it in the main window and record it strictly in this
;;;   workspace's isolated buffer list.
(def (marlin-ws-open-buffer state buffer-name)
  (let* ((ws (active-workspace state))
         (rule (marlin-ws-match-popup-rule state buffer-name)))
    (if rule
        (let ((updated (make-workspace
                         (.get ws id) (.get ws name) (.get ws project-root)
                         (.get ws active-buffer) (.get ws buffers)
                         buffer-name rule #t buffer-name (.get ws layout-mode))))
          (make-state (.get state active-id)
                      (replace-workspace (.get state workspaces) updated)
                      (.get state popup-rules)))
        (let* ((next-bufs (append-unique (.get ws buffers) buffer-name))
               (updated (make-workspace
                          (.get ws id) (.get ws name) (.get ws project-root)
                          buffer-name next-bufs
                          (.get ws popup-buffer) (.get ws popup-rule)
                          (.get ws popup-visible?) (.get ws last-popup-buffer)
                          (.get ws layout-mode))))
          (make-state (.get state active-id)
                      (replace-workspace (.get state workspaces) updated)
                      (.get state popup-rules))))))

;;; Dismiss the active workspace's popup drawer cleanly (Doom ESC / +popup/close)
;;; without altering the main window buffer or layout.
(def (marlin-ws-dismiss-popup state)
  (let ((ws (active-workspace state)))
    (if (not (.get ws popup-visible?))
        state
        (let* ((rule (.get ws popup-rule))
               (ephemeral? (and rule (eq? (.get rule ttl) 'ephemeral)))
               (remembered (if ephemeral? (.get ws last-popup-buffer) (.get ws popup-buffer)))
               (updated (make-workspace
                          (.get ws id) (.get ws name) (.get ws project-root)
                          (.get ws active-buffer) (.get ws buffers)
                          #f #f #f remembered (.get ws layout-mode))))
          (make-state (.get state active-id)
                      (replace-workspace (.get state workspaces) updated)
                      (.get state popup-rules))))))

;;; Toggle the active workspace's popup drawer visibility (Doom C-x p / +popup/toggle).
(def (marlin-ws-toggle-popup state)
  (let ((ws (active-workspace state)))
    (cond
      ((.get ws popup-visible?) (marlin-ws-dismiss-popup state))
      ((.get ws last-popup-buffer)
       (marlin-ws-open-buffer state (.get ws last-popup-buffer)))
      (else state))))

;;; Promote the active popup buffer into a first-class main window buffer inside
;;; the active workspace (Doom +popup/raise).
(def (marlin-ws-raise-popup state)
  (let ((ws (active-workspace state)))
    (if (not (and (.get ws popup-visible?) (.get ws popup-buffer)))
        state
        (let* ((promoted (.get ws popup-buffer))
               (next-bufs (append-unique (.get ws buffers) promoted))
               (updated (make-workspace
                          (.get ws id) (.get ws name) (.get ws project-root)
                          promoted next-bufs
                          #f #f #f #f (.get ws layout-mode))))
          (make-state (.get state active-id)
                      (replace-workspace (.get state workspaces) updated)
                      (.get state popup-rules))))))

;;; Return ONLY the buffers isolated to the active workspace (persp-mode).
(def (marlin-ws-buffers state)
  (.get (active-workspace state) buffers))

;;; Return the deduplicated union of buffers across all workspaces.
(def (marlin-ws-all-buffers state)
  (delete-duplicates/hash
    (apply append (map (lambda (ws) (.get ws buffers))
                       (.get state workspaces)))))

;;; Filter a candidate buffer list to the active workspace's isolated buffers
;;; unless include-all? is explicitly #t (persp-mode switch-buffer filtering).
(def (marlin-ws-isolate-candidates state candidates include-all?)
  (if include-all?
      candidates
      (let ((allowed (marlin-ws-buffers state)))
        (filter (lambda (b) (and (member b allowed) #t)) candidates))))

;;; Project workspace + popup state into a typed Scheme vector ready for
;;; native ABI projection (zero text/JSON serialization inside Scheme).
(def (marlin-ws-project-state state)
  (let* ((ws (active-workspace state))
         (rule (.get ws popup-rule))
         (popup-vec (if (and (.get ws popup-visible?) (.get ws popup-buffer) rule)
                        (vector 'popup
                                (.get ws popup-buffer)
                                (.get rule side)
                                (.get rule size)
                                (.get rule quit))
                        #f))
         (ws-vecs (list->vector
                    (map (lambda (w)
                           (vector 'workspace
                                   (.get w id)
                                   (.get w name)
                                   (.get w project-root)
                                   (.get w active-buffer)
                                   (list->vector (.get w buffers))))
                         (.get state workspaces)))))
    (vector 'marlin-workspace-state
            (.get state active-id)
            ws-vecs
            popup-vec
            (length (.get state popup-rules)))))

;;; Project all registered declarative PopupRules into a typed Scheme vector
;;; of #(popup-rule matcher side size select? quit ttl modeline?) tuples for
;;; native Rust ABI admission (zero text/JSON serialization inside Scheme).
(def (marlin-ws-project-popup-rules state)
  (list->vector
    (map (lambda (r)
           (vector 'popup-rule
                   (.get r matcher)
                   (.get r side)
                   (.get r size)
                   (.get r select?)
                   (.get r quit)
                   (.get r ttl)
                   (.get r modeline?)))
         (.get state popup-rules))))
