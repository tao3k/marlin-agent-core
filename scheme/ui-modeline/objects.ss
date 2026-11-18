;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .def .mix seal-object!))
(export NerdModelineIcons. DoomModeline PiModeline.)

;;; Code points from Nerd Fonts' Material Design collection.
(.def NerdModelineIcons.
  entries: '((modeline/left/brand . #xf023a)         ; fish
             (modeline/left/model . #xf167a)         ; robot-outline
             (modeline/left/activity . #xf05e1)      ; check-circle-outline
             (modeline/right/turns . #xf0199)        ; counter
             (modeline/right/session . #xf0efe)      ; identifier
             (modeline/state/online . #xf12cc)       ; cloud-check-outline
             (modeline/state/offline . #xf0164)      ; cloud-off-outline
             (modeline/state/unknown . #xf0a39)      ; cloud-question
             (modeline/state/runtime . #xf08bb)      ; cog-outline
             (modeline/state/workspace . #xf0256)    ; folder-outline
             (modeline/state/recovery . #xf099b)     ; restore
             (modeline/state/continuation . #xf05d6) ; alert-circle-outline
             (modeline/state/history . #xf02da)      ; history
             (modeline/state/working . #xf0996)      ; progress-clock
             (modeline/state/reading . #xf0c7d)))    ; file-search-outline

;;; Each slot records identity, label, tones, width limits and keep priority.
(.def UiModelineBase.
  activity-separator: "│ "
  slots: '((modeline/left/brand "MARLIN" muted muted 0 18 0)
           (modeline/left/model "{value}" muted muted 0 0 3)
           (modeline/left/activity "Ready" muted muted 0 64 0)))
(.def UiModelineSession.
  (slots => append '((modeline/right/turns "{value}" info info 72 0 1)
                     (modeline/right/session "{value}" muted muted 44 0 2)
                     (modeline/state/online "Online" success success 0 64 0)
                     (modeline/state/offline "Offline" warning warning 0 64 0)
                     (modeline/state/unknown "Unknown" warning warning 0 64 0)
                     (modeline/state/runtime "Runtime" info info 0 64 0)
                     (modeline/state/workspace "Workspace" info info 0 64 0)
                     (modeline/state/recovery "Recovery" warning warning 0 64 0)
                     (modeline/state/continuation "Continue blocked" warning warning 0 64 0)
                     (modeline/state/history "History" muted muted 0 64 0)
                     (modeline/state/working "Working" info info 0 64 0)
                     (modeline/state/reading "Reading" info info 0 64 0))))
(def DoomModeline
  (seal-object! (.mix UiModelineSession. UiModelineBase.)))

(.def PiModeline.
  activity-separator: "· "
  slots: '((modeline/left/brand "marlin" info info 0 16 0)
           (modeline/left/model "{value}" accent accent 0 0 1)
           (modeline/left/activity "ready" success warning 0 48 0)
           (modeline/right/session "{value}" muted muted 40 0 3)
           (modeline/right/turns "{value}" info info 66 0 2)
           (modeline/state/online "online" success success 0 48 0)
           (modeline/state/offline "offline" warning warning 0 48 0)
           (modeline/state/unknown "unknown" warning warning 0 48 0)
           (modeline/state/runtime "runtime" info info 0 48 0)
           (modeline/state/workspace "workspace" info info 0 48 0)
           (modeline/state/recovery "recovery" warning warning 0 48 0)
           (modeline/state/continuation "blocked" warning warning 0 48 0)
           (modeline/state/history "history" muted muted 0 48 0)
           (modeline/state/working "working" info info 0 48 0)
           (modeline/state/reading "reading" info info 0 48 0)))
