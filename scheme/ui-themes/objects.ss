;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Marlin's Doom One-inspired semantic palette. RGB channels cross the native
;;; boundary as Scheme integers; no terminal escape or text encoding lives here.
;;; Color references: doomemacs/themes themes/doom-one-theme.el at a5920291.
;;; The upstream repository is MIT licensed; no theme implementation is copied.
(import (only-in :clan/poo/object .def .mix seal-object!))
(export marlin-ui-doom-one-theme)

(.def UiThemeBase.
  canvas: '(40 44 52)
  ink: '(187 194 207)
  bar: '(28 31 36))
(.def UiDoomOne.
  identity: 'doom-one
  accent: '(198 120 221)
  info: '(81 175 239)
  success: '(152 190 101)
  warning: '(236 190 123)
  muted: '(115 121 126))
(def marlin-ui-doom-one-theme
  (seal-object! (.mix UiDoomOne. UiThemeBase.)))
