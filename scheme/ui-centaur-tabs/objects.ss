;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .def))
(export BuiltinTabs. NerdTabIcons. DoomTabs. PiTabs.)

(.def BuiltinTabs.
  entries: '((conversation "Chat" session "Chat")
             (messages "Messages" session "Msgs")
             (sessions "Sessions" session "Hist")
             (models "Models" workspace "Models")
             (processes "Processes" workspace "Procs")
             (help "Help" workspace "Help")))

(.def NerdTabIcons.
  entries: '((conversation . #xf086)
             (messages . #xf0e0)
             (sessions . #xf017)
             (models . #xf1b3)
             (processes . #xf085)
             (help . #xf059)))

(.def DoomTabs.
  selected-background: 'canvas
  selected-ink: 'normal
  unselected-ink: 'muted
  selected-mark: "▎"
  group-mark: "│")

(.def (PiTabs. @ DoomTabs.)
  selected-background: 'accent
  selected-ink: 'canvas
  selected-mark: "◆"
  group-mark: "·")
