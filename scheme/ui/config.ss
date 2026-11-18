;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/type element? validate)
        (only-in "../ui-components/config.ss" UiComponentsScope)
        (only-in "../ui-modeline/config.ss" UiModelineScope)
        (only-in "../ui-transcript/config.ss" UiTranscriptScope)
        (only-in "../ui-themes/config.ss" UiThemesScope)
        (only-in "../ui-layout/config.ss" UiLayoutScope)
        (only-in "../ui-components/types.ss" UiComponentsScopeContract)
        (only-in "../ui-modeline/types.ss" UiModelineScopeContract)
        (only-in "../ui-transcript/types.ss" UiTranscriptScopeContract)
        (only-in "../ui-themes/types.ss" UiThemesScopeContract)
        (only-in "../ui-layout/types.ss" UiLayoutScopeContract)
        "./types.ss" "./objects.ss")
(export make-ui-scope UiScope)
(def (make-ui-scope components modeline transcript themes layout
                    method-factories: (method-factories #()))
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
  (unless (ui-method-factories? method-factories)
    (error "invalid UI package method factories"))
  ;; Capture the declarations, not a caller-mutable vector. The factories
  ;; themselves build fresh method objects for each session.
  (def factories (vector-copy method-factories))
  (validate UiScopeContract
    (make-ui-operations components modeline transcript themes layout factories)))

;;; Native imports express the selected source dependency graph. Package
;;; contributions extend this immutable base rather than a second registry.
(def UiScope
  (make-ui-scope UiComponentsScope UiModelineScope UiTranscriptScope
                 UiThemesScope UiLayoutScope))
