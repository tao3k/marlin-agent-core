;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure functional operations for Marlin's Doom-inspired Workspace isolation
;;; (persp-mode), recursive multi-window tree (winum + winner-mode + dedicated
;;; windows), Doom modeline window-number projection, and declarative Popup
;;; Window Rule manager (set-popup-rule!).
(import (only-in :std/list/list delete-duplicates/hash filter)
        (only-in :clan/poo/object .get .o seal-object!)
        (only-in "./objects.ss" PopupRule. WindowLeaf. WindowSplit. Workspace. DefaultPopupRules))
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
        marlin-ws-window-list
        marlin-ws-window-count
        marlin-ws-focused-window
        marlin-ws-window-number
        marlin-ws-focused-window-number
        marlin-ws-split-window
        marlin-ws-select-window-number
        marlin-ws-other-window
        marlin-ws-delete-window
        marlin-ws-delete-other-windows
        marlin-ws-swap-windows
        marlin-ws-resize-window
        marlin-ws-balance-windows
        marlin-ws-toggle-dedicated
        marlin-ws-winner-undo
        marlin-ws-winner-redo
        marlin-ws-modeline-window-indicator
        marlin-ws-project-window-tree
        marlin-ws-project-state
        marlin-ws-project-popup-rules)

(def (clamp-ratio size)
  (cond
    ((not (number? size)) 0.35)
    ((< size 0.15) 0.15)
    ((> size 0.70) 0.70)
    (else size)))

(def (clamp-split-ratio ratio)
  (cond
    ((not (number? ratio)) 0.50)
    ((< ratio 0.15) 0.15)
    ((> ratio 0.85) 0.85)
    (else ratio)))

(def (valid-side side)
  (if (memq side '(bottom right left top)) side 'bottom))

(def (valid-quit quit)
  (if (memq quit '(current other never)) quit 'current))

(def (valid-ttl ttl)
  (if (memq ttl '(ephemeral session persistent)) ttl 'ephemeral))

(def (valid-direction dir)
  (if (eq? dir 'horizontal) 'horizontal 'vertical))

(def (append-unique lst item)
  (if (member item lst)
      lst
      (append lst (list item))))

(def (make-window-leaf id buffer-name dedicated?)
  (seal-object!
    (.o (:: @ WindowLeaf.)
        (kind := 'leaf)
        (id := id)
        (buffer := buffer-name)
        (dedicated? := dedicated?))))

(def (make-window-split direction ratio first-node second-node)
  (seal-object!
    (.o (:: @ WindowSplit.)
        (kind := 'split)
        (direction := (valid-direction direction))
        (ratio := (clamp-split-ratio ratio))
        (first := first-node)
        (second := second-node))))

(def (leaf-node? node)
  (and node (eq? (.get node kind) 'leaf)))

(def (split-node? node)
  (and node (eq? (.get node kind) 'split)))

(def (tree-leaves node)
  (cond
    ((not node) '())
    ((leaf-node? node) (list node))
    ((split-node? node)
     (append (tree-leaves (.get node first))
             (tree-leaves (.get node second))))
    (else '())))

(def (tree-contains-id? node target-id)
  (cond
    ((not node) #f)
    ((leaf-node? node) (equal? (.get node id) target-id))
    ((split-node? node)
     (or (tree-contains-id? (.get node first) target-id)
         (tree-contains-id? (.get node second) target-id)))
    (else #f)))

(def (tree-find-leaf node target-id)
  (cond
    ((not node) #f)
    ((leaf-node? node) (if (equal? (.get node id) target-id) node #f))
    ((split-node? node)
     (or (tree-find-leaf (.get node first) target-id)
         (tree-find-leaf (.get node second) target-id)))
    (else #f)))

(def (tree-map-leaf node target-id fn)
  (cond
    ((not node) #f)
    ((leaf-node? node)
     (if (equal? (.get node id) target-id) (fn node) node))
    ((split-node? node)
     (make-window-split (.get node direction)
                        (.get node ratio)
                        (tree-map-leaf (.get node first) target-id fn)
                        (tree-map-leaf (.get node second) target-id fn)))
    (else node)))

(def (tree-delete-leaf node target-id)
  (cond
    ((not node) #f)
    ((leaf-node? node)
     (if (equal? (.get node id) target-id) #f node))
    ((split-node? node)
     (let* ((left (.get node first))
            (right (.get node second))
            (new-left (tree-delete-leaf left target-id))
            (new-right (tree-delete-leaf right target-id)))
       (cond
         ((and (not new-left) (not new-right)) #f)
         ((not new-left) new-right)
         ((not new-right) new-left)
         (else (make-window-split (.get node direction)
                                  (.get node ratio)
                                  new-left
                                  new-right)))))
    (else node)))

;;; Count consecutive pane spans along the same split axis (Ghostty / tmux /
;;; Emacs window-combination-resize proportional auto-balance weighting).
(def (tree-axis-weight node dir)
  (cond
    ((not node) 0)
    ((leaf-node? node) 1)
    ((and (split-node? node) (eq? (.get node direction) dir))
     (+ (tree-axis-weight (.get node first) dir)
        (tree-axis-weight (.get node second) dir)))
    (else 1)))

(def (tree-balance node)
  (cond
    ((not node) #f)
    ((leaf-node? node) node)
    ((split-node? node)
     (let* ((dir (.get node direction))
            (left (tree-balance (.get node first)))
            (right (tree-balance (.get node second)))
            (w1 (tree-axis-weight left dir))
            (w2 (tree-axis-weight right dir))
            (total (+ w1 w2))
            (ratio (if (> total 0) (/ (exact->inexact w1) (exact->inexact total)) 0.50)))
       (make-window-split dir ratio left right)))
    (else node)))

;;; Adjust the ratio of the innermost split containing target-id.
(def (tree-resize-enclosing node target-id delta)
  (cond
    ((not node) (values #f #f))
    ((leaf-node? node) (values node #f))
    ((split-node? node)
     (let* ((left (.get node first))
            (right (.get node second))
            (in-left? (tree-contains-id? left target-id))
            (in-right? (tree-contains-id? right target-id)))
       (cond
         ((and (not in-left?) (not in-right?)) (values node #f))
         (in-left?
          (let-values (((next-left handled?) (tree-resize-enclosing left target-id delta)))
            (if handled?
                (values (make-window-split (.get node direction) (.get node ratio) next-left right) #t)
                (values (make-window-split (.get node direction)
                                           (clamp-split-ratio (+ (.get node ratio) delta))
                                           left right)
                        #t))))
         (else
          (let-values (((next-right handled?) (tree-resize-enclosing right target-id delta)))
            (if handled?
                (values (make-window-split (.get node direction) (.get node ratio) left next-right) #t)
                (values (make-window-split (.get node direction)
                                           (clamp-split-ratio (- (.get node ratio) delta))
                                           left right)
                        #t)))))))
    (else (values node #f))))

(def (tree-layout-mode node)
  (if (split-node? node) (.get node direction) 'single))

(def (make-workspace id name project-root active-buffer buffers
                     popup-buffer popup-rule popup-visible? last-popup-buffer
                     layout-mode window-tree focused-window-id next-window-seq
                     winner-undo winner-redo)
  (let* ((tree (or window-tree (make-window-leaf "w1" active-buffer #f)))
         (leaves (tree-leaves tree))
         (focus-leaf (or (tree-find-leaf tree focused-window-id)
                         (and (pair? leaves) (car leaves))
                         (make-window-leaf "w1" active-buffer #f)))
         (focus-id (.get focus-leaf id))
         (focus-buf (.get focus-leaf buffer))
         (derived-mode (tree-layout-mode tree)))
    (seal-object!
      (.o (:: @ Workspace.)
          (id := id)
          (name := name)
          (project-root := project-root)
          (active-buffer := focus-buf)
          (buffers := buffers)
          (popup-buffer := popup-buffer)
          (popup-rule := popup-rule)
          (popup-visible? := popup-visible?)
          (last-popup-buffer := last-popup-buffer)
          (layout-mode := (if (eq? layout-mode 'single) derived-mode derived-mode))
          (window-tree := tree)
          (focused-window-id := focus-id)
          (next-window-seq := next-window-seq)
          (winner-undo := winner-undo)
          (winner-redo := winner-redo)))))

(def (workspace-with-tree ws next-tree next-focus-id next-seq next-bufs record-winner?)
  (let* ((max-history 24)
         (prev-snap (cons (.get ws window-tree) (.get ws focused-window-id)))
         (undo-stack (if record-winner?
                         (let ((pushed (cons prev-snap (.get ws winner-undo))))
                           (if (> (length pushed) max-history)
                               (let take-loop ((lst pushed) (n max-history) (acc '()))
                                 (if (or (<= n 0) (null? lst))
                                     (reverse acc)
                                     (take-loop (cdr lst) (- n 1) (cons (car lst) acc))))
                               pushed))
                         (.get ws winner-undo)))
         (redo-stack (if record-winner? '() (.get ws winner-redo))))
    (make-workspace
      (.get ws id) (.get ws name) (.get ws project-root)
      (.get ws active-buffer) next-bufs
      (.get ws popup-buffer) (.get ws popup-rule)
      (.get ws popup-visible?) (.get ws last-popup-buffer)
      (tree-layout-mode next-tree)
      next-tree next-focus-id next-seq undo-stack redo-stack)))

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

(def (index-of-first pred lst)
  (let loop ((rest lst) (idx 0))
    (cond
      ((null? rest) -1)
      ((pred (car rest)) idx)
      (else (loop (cdr rest) (+ idx 1))))))

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

(def (default-secondary-buffer primary)
  (cond
    ((equal? primary "*conversation*") "*Sessions*")
    ((equal? primary "*Dashboard*") "*conversation*")
    (else "*conversation*")))

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
                          #f #f #f #f 'single
                          (make-window-leaf "w1" "*Dashboard*" #f)
                          "w1" 1 '() '()))
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
                                   #f #f #f #f 'single
                                   (make-window-leaf "w1" initial-buf #f)
                                   "w1" 1 '() '())))
          (make-state id
                      (append (.get state workspaces) (list ws))
                      (.get state popup-rules))))))

;;; Switch active workspace, preserving each workspace's exact buffer list,
;;; window tree, focused window, winner-mode history, and popup drawer state.
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
;;; - If governed by a PopupRule and not already promoted, mount it into the
;;;   workspace's isolated popup drawer WITHOUT disturbing windows or tabs.
;;; - If the focused window is strongly dedicated to another buffer (Emacs
;;;   `toggle-window-dedicated`), route to an undedicated sibling window or
;;;   open a vertical split rather than overwriting the dedicated window.
;;; - Otherwise display it in the focused window and record it in this
;;;   workspace's isolated buffer list.
(def (marlin-ws-open-buffer state buffer-name)
  (let* ((ws (active-workspace state))
         (rule (marlin-ws-match-popup-rule state buffer-name))
         (already-main? (and (member buffer-name (.get ws buffers)) #t)))
    (if (and rule (not already-main?))
        (let ((updated (make-workspace
                         (.get ws id) (.get ws name) (.get ws project-root)
                         (.get ws active-buffer) (.get ws buffers)
                         buffer-name rule #t buffer-name (.get ws layout-mode)
                         (.get ws window-tree) (.get ws focused-window-id)
                         (.get ws next-window-seq)
                         (.get ws winner-undo) (.get ws winner-redo))))
          (make-state (.get state active-id)
                      (replace-workspace (.get state workspaces) updated)
                      (.get state popup-rules)))
        (let* ((focus-id (.get ws focused-window-id))
               (focus-leaf (tree-find-leaf (.get ws window-tree) focus-id))
               (strongly-dedicated?
                 (and focus-leaf
                      (eq? (.get focus-leaf dedicated?) 'strong)
                      (not (equal? (.get focus-leaf buffer) buffer-name))))
               (next-bufs (append-unique (.get ws buffers) buffer-name)))
          (if strongly-dedicated?
              (let ((other-leaf (find-first (lambda (w) (not (eq? (.get w dedicated?) 'strong)))
                                            (tree-leaves (.get ws window-tree)))))
                (if other-leaf
                    (let* ((target-id (.get other-leaf id))
                           (next-tree (tree-map-leaf (.get ws window-tree) target-id
                                                     (lambda (leaf)
                                                       (make-window-leaf (.get leaf id) buffer-name (.get leaf dedicated?)))))
                           (updated (workspace-with-tree ws next-tree target-id (.get ws next-window-seq) next-bufs #f)))
                      (make-state (.get state active-id)
                                  (replace-workspace (.get state workspaces) updated)
                                  (.get state popup-rules)))
                    (marlin-ws-split-window state 'vertical buffer-name)))
              (let* ((next-tree (tree-map-leaf (.get ws window-tree) focus-id
                                               (lambda (leaf)
                                                 (make-window-leaf (.get leaf id) buffer-name (.get leaf dedicated?)))))
                     (updated (workspace-with-tree ws next-tree focus-id (.get ws next-window-seq) next-bufs #f)))
                (make-state (.get state active-id)
                            (replace-workspace (.get state workspaces) updated)
                            (.get state popup-rules))))))))

;;; Dismiss the active workspace's popup drawer cleanly (Doom ESC / +popup/close)
;;; without altering the main window tree or layout.
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
                          #f #f #f remembered (.get ws layout-mode)
                          (.get ws window-tree) (.get ws focused-window-id)
                          (.get ws next-window-seq)
                          (.get ws winner-undo) (.get ws winner-redo))))
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

;;; Promote the active popup buffer into the focused main window inside the
;;; active workspace (Doom +popup/raise).
(def (marlin-ws-raise-popup state)
  (let ((ws (active-workspace state)))
    (if (not (and (.get ws popup-visible?) (.get ws popup-buffer)))
        state
        (let* ((promoted (.get ws popup-buffer))
               (next-bufs (append-unique (.get ws buffers) promoted))
               (focus-id (.get ws focused-window-id))
               (next-tree (tree-map-leaf (.get ws window-tree) focus-id
                                         (lambda (leaf)
                                           (make-window-leaf (.get leaf id) promoted #f))))
               (updated (make-workspace
                          (.get ws id) (.get ws name) (.get ws project-root)
                          promoted next-bufs
                          #f #f #f #f (tree-layout-mode next-tree)
                          next-tree focus-id (.get ws next-window-seq)
                          (.get ws winner-undo) (.get ws winner-redo))))
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

;;; Return deterministic reading-order (left-to-right, top-to-bottom) list of
;;; live WindowLeaf objects in the active workspace (winum ordering).
(def (marlin-ws-window-list state)
  (tree-leaves (.get (active-workspace state) window-tree)))

;;; Return total live window count in the active workspace.
(def (marlin-ws-window-count state)
  (length (marlin-ws-window-list state)))

;;; Return the focused WindowLeaf in the active workspace.
(def (marlin-ws-focused-window state)
  (let ((ws (active-workspace state)))
    (or (tree-find-leaf (.get ws window-tree) (.get ws focused-window-id))
        (car (marlin-ws-window-list state)))))

;;; Return the 1-based winum number for window-id in the active workspace (0 if absent).
(def (marlin-ws-window-number state window-id)
  (let ((idx (index-of-first (lambda (w) (equal? (.get w id) window-id))
                             (marlin-ws-window-list state))))
    (if (< idx 0) 0 (+ idx 1))))

;;; Return the 1-based winum number of the focused window.
(def (marlin-ws-focused-window-number state)
  (marlin-ws-window-number state (.get (active-workspace state) focused-window-id)))

;;; Split the focused window vertically (left | right) or horizontally (top / bottom),
;;; defaulting the new pane to the focused window's current buffer (Emacs /
;;; Ghostty / tmux behavior) and auto-balancing split proportions across the axis.
(def (marlin-ws-split-window state direction secondary-buffer)
  (let* ((ws (active-workspace state))
         (focus-id (.get ws focused-window-id))
         (focus-leaf (marlin-ws-focused-window state))
         (buf (if (and (string? secondary-buffer) (> (string-length secondary-buffer) 0))
                  secondary-buffer
                  (.get focus-leaf buffer)))
         (next-seq (+ (.get ws next-window-seq) 1))
         (new-id (string-append "w" (number->string next-seq)))
         (new-leaf (make-window-leaf new-id buf #f))
         (raw-tree (tree-map-leaf (.get ws window-tree) focus-id
                                  (lambda (leaf)
                                    (make-window-split direction 0.50 leaf new-leaf))))
         (next-tree (tree-balance raw-tree))
         (next-bufs (append-unique (.get ws buffers) buf))
         (updated (workspace-with-tree ws next-tree new-id next-seq next-bufs #t)))
    (make-state (.get state active-id)
                (replace-workspace (.get state workspaces) updated)
                (.get state popup-rules))))

;;; Select window by 1-based winum index (Doom SPC w 1..9 / winum-select-window-N).
(def (marlin-ws-select-window-number state num)
  (let ((leaves (marlin-ws-window-list state)))
    (if (or (< num 1) (> num (length leaves)))
        state
        (let* ((ws (active-workspace state))
               (target (list-ref leaves (- num 1)))
               (updated (workspace-with-tree ws (.get ws window-tree) (.get target id)
                                             (.get ws next-window-seq) (.get ws buffers) #f)))
          (make-state (.get state active-id)
                      (replace-workspace (.get state workspaces) updated)
                      (.get state popup-rules))))))

;;; Cycle focus forward (+1, SPC w w) or backward (-1, SPC w W) in winum order.
(def (marlin-ws-other-window state delta)
  (let* ((leaves (marlin-ws-window-list state))
         (n (length leaves)))
    (if (<= n 1)
        state
        (let* ((cur (- (marlin-ws-focused-window-number state) 1))
               (raw (modulo (+ cur delta) n))
               (next-idx (if (< raw 0) (+ raw n) raw)))
          (marlin-ws-select-window-number state (+ next-idx 1))))))

;;; Doom +workspace/close-window-or-workspace (SPC w d):
;;; - When > 1 windows exist in the active workspace, close the focused window
;;;   and record the previous layout in winner-undo.
;;; - When only 1 window remains in a non-main workspace and other workspaces
;;;   exist, kill the workspace (matching Doom Emacs persp-mode).
(def (marlin-ws-delete-window state)
  (let* ((ws (active-workspace state))
         (leaves (marlin-ws-window-list state)))
    (if (<= (length leaves) 1)
        (if (and (> (length (.get state workspaces)) 1)
                 (not (equal? (.get ws id) "main")))
            (marlin-ws-kill state (.get ws id))
            state)
        (let* ((focus-id (.get ws focused-window-id))
               (cur-idx (index-of-first (lambda (w) (equal? (.get w id) focus-id)) leaves))
               (next-tree (tree-balance (tree-delete-leaf (.get ws window-tree) focus-id)))
               (next-leaves (tree-leaves next-tree))
               (next-idx (min cur-idx (- (length next-leaves) 1)))
               (next-focus (.get (list-ref next-leaves (max 0 next-idx)) id))
               (updated (workspace-with-tree ws next-tree next-focus (.get ws next-window-seq) (.get ws buffers) #t)))
          (make-state (.get state active-id)
                      (replace-workspace (.get state workspaces) updated)
                      (.get state popup-rules))))))

;;; Collapse all splits to a single window keeping the focused buffer (Doom SPC w o / C-x 1).
(def (marlin-ws-delete-other-windows state)
  (let* ((ws (active-workspace state))
         (leaves (marlin-ws-window-list state)))
    (if (<= (length leaves) 1)
        state
        (let* ((focus-leaf (marlin-ws-focused-window state))
               (single-leaf (make-window-leaf (.get focus-leaf id)
                                              (.get focus-leaf buffer)
                                              (.get focus-leaf dedicated?)))
               (updated (workspace-with-tree ws single-leaf (.get single-leaf id)
                                             (.get ws next-window-seq) (.get ws buffers) #t)))
          (make-state (.get state active-id)
                      (replace-workspace (.get state workspaces) updated)
                      (.get state popup-rules))))))

;;; Swap the focused window's buffer with the next window in winum order (Doom SPC w x).
(def (marlin-ws-swap-windows state)
  (let* ((ws (active-workspace state))
         (leaves (marlin-ws-window-list state))
         (n (length leaves)))
    (if (<= n 1)
        state
        (let* ((cur-idx (- (marlin-ws-focused-window-number state) 1))
               (next-idx (modulo (+ cur-idx 1) n))
               (a (list-ref leaves cur-idx))
               (b (list-ref leaves next-idx))
               (a-id (.get a id))
               (b-id (.get b id))
               (a-buf (.get a buffer))
               (b-buf (.get b buffer))
               (t1 (tree-map-leaf (.get ws window-tree) a-id
                                  (lambda (leaf) (make-window-leaf a-id b-buf (.get leaf dedicated?)))))
               (t2 (tree-map-leaf t1 b-id
                                  (lambda (leaf) (make-window-leaf b-id a-buf (.get leaf dedicated?)))))
               (updated (workspace-with-tree ws t2 a-id (.get ws next-window-seq) (.get ws buffers) #t)))
          (make-state (.get state active-id)
                      (replace-workspace (.get state workspaces) updated)
                      (.get state popup-rules))))))

;;; Resize the split enclosing the focused window by delta (clamped to [0.20, 0.80]).
(def (marlin-ws-resize-window state delta)
  (let ((ws (active-workspace state)))
    (if (<= (marlin-ws-window-count state) 1)
        state
        (let-values (((next-tree handled?)
                      (tree-resize-enclosing (.get ws window-tree) (.get ws focused-window-id) delta)))
          (if (not handled?)
              state
              (let ((updated (workspace-with-tree ws next-tree (.get ws focused-window-id)
                                                  (.get ws next-window-seq) (.get ws buffers) #f)))
                (make-state (.get state active-id)
                            (replace-workspace (.get state workspaces) updated)
                            (.get state popup-rules))))))))

;;; Reset all split ratios in the active workspace to 0.50 (Doom SPC w e / C-x +).
(def (marlin-ws-balance-windows state)
  (let ((ws (active-workspace state)))
    (if (<= (marlin-ws-window-count state) 1)
        state
        (let* ((balanced (tree-balance (.get ws window-tree)))
               (updated (workspace-with-tree ws balanced (.get ws focused-window-id)
                                             (.get ws next-window-seq) (.get ws buffers) #t)))
          (make-state (.get state active-id)
                      (replace-workspace (.get state workspaces) updated)
                      (.get state popup-rules))))))

;;; Toggle strong window dedication on the focused window (Emacs 30 toggle-window-dedicated).
(def (marlin-ws-toggle-dedicated state)
  (let* ((ws (active-workspace state))
         (focus-id (.get ws focused-window-id))
         (next-tree (tree-map-leaf (.get ws window-tree) focus-id
                                   (lambda (leaf)
                                     (make-window-leaf (.get leaf id)
                                                       (.get leaf buffer)
                                                       (if (.get leaf dedicated?) #f 'strong)))))
         (updated (workspace-with-tree ws next-tree focus-id (.get ws next-window-seq) (.get ws buffers) #f)))
    (make-state (.get state active-id)
                (replace-workspace (.get state workspaces) updated)
                (.get state popup-rules))))

;;; Per-workspace winner-mode undo (Doom SPC w u / +workspaces-save-winner-data-h).
(def (marlin-ws-winner-undo state)
  (let* ((ws (active-workspace state))
         (undo (.get ws winner-undo)))
    (if (null? undo)
        state
        (let* ((prev (car undo))
               (cur-snap (cons (.get ws window-tree) (.get ws focused-window-id)))
               (next-tree (car prev))
               (next-focus (cdr prev))
               (updated (make-workspace
                          (.get ws id) (.get ws name) (.get ws project-root)
                          (.get ws active-buffer) (.get ws buffers)
                          (.get ws popup-buffer) (.get ws popup-rule)
                          (.get ws popup-visible?) (.get ws last-popup-buffer)
                          (tree-layout-mode next-tree)
                          next-tree next-focus (.get ws next-window-seq)
                          (cdr undo) (cons cur-snap (.get ws winner-redo)))))
          (make-state (.get state active-id)
                      (replace-workspace (.get state workspaces) updated)
                      (.get state popup-rules))))))

;;; Per-workspace winner-mode redo (Doom SPC w U).
(def (marlin-ws-winner-redo state)
  (let* ((ws (active-workspace state))
         (redo (.get ws winner-redo)))
    (if (null? redo)
        state
        (let* ((nxt (car redo))
               (cur-snap (cons (.get ws window-tree) (.get ws focused-window-id)))
               (next-tree (car nxt))
               (next-focus (cdr nxt))
               (updated (make-workspace
                          (.get ws id) (.get ws name) (.get ws project-root)
                          (.get ws active-buffer) (.get ws buffers)
                          (.get ws popup-buffer) (.get ws popup-rule)
                          (.get ws popup-visible?) (.get ws last-popup-buffer)
                          (tree-layout-mode next-tree)
                          next-tree next-focus (.get ws next-window-seq)
                          (cons cur-snap (.get ws winner-undo)) (cdr redo))))
          (make-state (.get state active-id)
                      (replace-workspace (.get state workspaces) updated)
                      (.get state popup-rules))))))

;;; Mirror doom-modeline--unicode-number verbatim:
;;; returns "" when <= 1 live window (never clutters single-window modeline
;;; with "W1"), or "➊".."➓" / "1".."10" when multiple windows are open.
(def (doom-unicode-number num)
  (case num
    ((1) "➊")
    ((2) "➋")
    ((3) "➌")
    ((4) "➍")
    ((5) "➎")
    ((6) "➏")
    ((7) "➐")
    ((8) "➑")
    ((9) "➒")
    ((10) "➓")
    (else (number->string num))))

(def (marlin-ws-modeline-window-indicator state window-id unicode?)
  (if (<= (marlin-ws-window-count state) 1)
      ""
      (let ((num (marlin-ws-window-number state window-id)))
        (if (<= num 0)
            ""
            (if unicode?
                (doom-unicode-number num)
                (number->string num))))))

(def (project-node node state)
  (cond
    ((leaf-node? node)
     (vector 'leaf
             (.get node id)
             (marlin-ws-window-number state (.get node id))
             (.get node buffer)
             (.get node dedicated?)))
    ((split-node? node)
     (vector 'split
             (.get node direction)
             (.get node ratio)
             (project-node (.get node first) state)
             (project-node (.get node second) state)))
    (else #f)))

;;; Project the active workspace's multi-window tree into a typed Scheme vector
;;; for native Rust/UI ABI consumption (zero text/JSON serialization inside Scheme).
(def (marlin-ws-project-window-tree state)
  (let ((ws (active-workspace state)))
    (vector 'window-tree
            (.get ws focused-window-id)
            (marlin-ws-window-count state)
            (project-node (.get ws window-tree) state))))

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
