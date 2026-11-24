;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .def))
(export BuiltinTabs. NerdTabIcons. TabStyles. TaskTabBadges.)

(.def BuiltinTabs.
  entries: '((conversation "Chat" session "Chat")
             (messages "Messages" session "Msgs")
             (sessions "Sessions" session "Hist")
             (models "Models" workspace "Models")
             (processes "Processes" workspace "Procs")
             (help "Help" workspace "Help")))

(.def NerdTabIcons.
  entries: '((conversation #xf086 info)
             (messages #xf0e0 warning)
             (sessions #xf017 success)
             (models #xf1b3 accent)
             (processes #xf085 info)
             (help #xf059 warning)
             (index #xf022 accent)))

(.def TaskTabBadges.
  mode-suffix: '((active . "")
                 (peek-readonly . " [peek]")
                 (archived-focused-until-blur . " [arch]"))
  status-mark: '((todo . "○ ")
                 (in-progress . "◐ ")
                 (blocked . "⊘ ")
                 (review . "◈ ")
                 (done . "● ")))

(.def BarTabs.
  selected-background: 'canvas
  selected-ink: 'normal
  selected-icon-ink: #f
  unselected-ink: 'muted
  selected-mark: "▎"
  selected-end: " "
  group-mark: "│")

(.def (BoxTabs. @ BarTabs.)
  selected-mark: "│"
  selected-end: "│")

(.def (SlantTabs. @ BarTabs.)
  selected-background: 'accent
  selected-ink: 'canvas
  selected-icon-ink: 'canvas
  selected-mark: "╱"
  selected-end: "╱"
  group-mark: "·")

(.def TabStyles.
  entries: (list (list 'tabs-bar "Slim selection mark" BarTabs.)
                 (list 'tabs-box "Outlined selected tab" BoxTabs.)
                 (list 'tabs-slant "Angled edges and accent fill" SlantTabs.)))
