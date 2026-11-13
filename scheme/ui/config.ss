;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/mop element? validate)
        (only-in "../ui-components/types.ss" UiComponentsScopeContract)
        (only-in "../ui-modeline/types.ss" UiModelineScopeContract)
        (only-in "../ui-transcript/types.ss" UiTranscriptScopeContract)
        (only-in "../ui-themes/types.ss" UiThemesScopeContract)
        (only-in "../ui-layout/types.ss" UiLayoutScopeContract)
        "./types.ss" "./objects.ss")
(export make-ui-scope)
(def (make-ui-scope components modeline transcript themes layout)
  (unless (element? UiComponentsScopeContract components)
    (error "invalid selected UI components scope"))
  (unless (element? UiModelineScopeContract modeline)
    (error "invalid selected UI modeline scope"))
  (unless (element? UiTranscriptScopeContract transcript)
    (error "invalid selected UI transcript scope"))
  (unless (element? UiThemesScopeContract themes)
    (error "invalid selected UI themes scope"))
  (unless (element? UiLayoutScopeContract layout)
    (error "invalid selected UI layout scope"))
  (validate UiScopeContract (make-ui-operations components modeline transcript themes layout)))
