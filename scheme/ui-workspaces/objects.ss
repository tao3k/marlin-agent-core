;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; POO prototypes for Doom Emacs-style popup rules (set-popup-rule!) and
;;; isolated perspective workspaces (persp-mode / :ui workspaces).
(import (only-in :clan/poo/object .def .o seal-object!))
(export PopupRule. Workspace. DefaultPopupRules)

;;; Base POO prototype for a declarative popup window rule.
(.def PopupRule.
  matcher: ""
  side: 'bottom
  size: 0.35
  slot: 0
  select?: #t
  quit: 'current
  ttl: 'ephemeral
  modeline?: #f)

;;; Base POO prototype for one isolated perspective workspace.
(.def Workspace.
  id: "main"
  name: "main"
  project-root: ""
  active-buffer: "*Dashboard*"
  buffers: '("*Dashboard*" "*conversation*")
  popup-buffer: #f
  popup-rule: #f
  popup-visible?: #f
  last-popup-buffer: #f
  layout-mode: 'single)

;;; Curated Doom Emacs popup rules keeping auxiliary buffers out of the main
;;; editing/conversation layout while preserving instant dismiss & raise.
(def DefaultPopupRules
  (list
    (seal-object! (.o (:: @ PopupRule.)
                      matcher: "*Help*"
                      side: 'bottom
                      size: 0.36
                      slot: 1
                      select?: #t
                      quit: 'current
                      ttl: 'ephemeral
                      modeline?: #f))
    (seal-object! (.o (:: @ PopupRule.)
                      matcher: "*Messages*"
                      side: 'bottom
                      size: 0.30
                      slot: 2
                      select?: #f
                      quit: 'current
                      ttl: 'ephemeral
                      modeline?: #f))
    (seal-object! (.o (:: @ PopupRule.)
                      matcher: "*Processes*"
                      side: 'bottom
                      size: 0.34
                      slot: 3
                      select?: #t
                      quit: 'current
                      ttl: 'session
                      modeline?: #f))
    (seal-object! (.o (:: @ PopupRule.)
                      matcher: "*Models*"
                      side: 'right
                      size: 0.38
                      slot: 1
                      select?: #t
                      quit: 'current
                      ttl: 'session
                      modeline?: #f))
    (seal-object! (.o (:: @ PopupRule.)
                      matcher: "*Targets*"
                      side: 'right
                      size: 0.40
                      slot: 2
                      select?: #t
                      quit: 'current
                      ttl: 'session
                      modeline?: #f))
    (seal-object! (.o (:: @ PopupRule.)
                      matcher: "*Artifact*"
                      side: 'right
                      size: 0.44
                      slot: 0
                      select?: #t
                      quit: 'other
                      ttl: 'session
                      modeline?: #f))))
