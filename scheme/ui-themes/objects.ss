;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Marlin's Doom One-inspired semantic palette. RGB channels cross the native
;;; boundary as Scheme integers; no terminal escape or text encoding lives here.
;;; Color references: doomemacs/themes themes/doom-one-theme.el at a5920291.
;;; The upstream repository is MIT licensed; no theme implementation is copied.
(import (only-in :clan/poo/object .def .mix seal-object!))
(export marlin-ui-doom-one-theme marlin-ui-theme-for
        marlin-ui-theme-profiles marlin-ui-default-profile)

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

(.def UiPiInspired.
  identity: 'pi
  canvas: '(15 17 24)
  ink: '(210 214 224)
  bar: '(23 26 36)
  accent: '(255 186 104)
  info: '(75 192 224)
  success: '(84 206 157)
  warning: '(245 169 103)
  muted: '(134 143 157))
(def pi-inspired-theme (seal-object! UiPiInspired.))

;;; One profile registry supplies the M-x name, description, default and
;;; palette. The modeline selector is checked when a profile is opened.
(def marlin-ui-theme-profiles
  (list (list 'doom-one "Doom One Modeline" marlin-ui-doom-one-theme)
        (list 'pi "oh-my-pi-inspired Modeline" pi-inspired-theme)))
(def marlin-ui-default-profile (caar marlin-ui-theme-profiles))

(def (marlin-ui-theme-for profile)
  (let (entry (assq profile marlin-ui-theme-profiles))
    (if entry (caddr entry) (error "unknown UI theme profile"))))
