;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure value-level composition and native Scheme vector projection for
;;; Marlin MyGo surfaces (Desktop Split + Mobile Stack).
(import (only-in :clan/poo/object .get .mix .o seal-object!)
        (only-in :clan/poo/type validate)
        (only-in :std/list/list find)
        (only-in "../ui-centaur-tabs/config.ss" UiCentaurTabsScope)
        (only-in "../ui-components/config.ss" UiComponentsScope)
        (only-in "../ui-elements/funcs.ss"
                 ui-columns ui-panel ui-region ui-rows ui-scroll-target
                 ui-size-cells ui-size-flex ui-sized-with ui-slot ui-template)
        (only-in "../ui-layout/minibuffer.ss" minibuffer-echo-policy)
        (only-in "../ui-layout/objects.ss" UiLayoutBase.)
        (only-in "../ui-layout/types.ss" UiLayoutScopeContract)
        (only-in "../ui-modeline/config.ss" UiModelineScope)
        (only-in "../ui-themes/config.ss" UiThemesScope)
        (only-in "../ui-transcript/config.ss" UiTranscriptScope)
        (only-in "../ui/config.ss" make-ui-scope)
        "./objects.ss"
        "./types.ss")
(export marlin-mygo-profile-for
        marlin-mygo-layout-policy-for
        marlin-mygo-layout-scope-for
        marlin-mygo-slot-declarations-for
        marlin-mygo-modules-for
        marlin-mygo-admit-plugin-slot?
        marlin-mygo-project-overlay
        marlin-mygo-project-surface-manifest
        marlin-mygo-make-ui-scope)

(def mygo-panel-title
  (ui-template 'none '((literal normal #f " ") (binding normal #t panel-title))))
(def mygo-panel-field
  (ui-template 'none '((binding muted #f label) (literal muted #f ": ")
                       (binding normal #f value))))

(def (marlin-mygo-profile-for target-kind)
  (validate MygoTargetKind target-kind)
  (validate MygoProfileOverlayContract
    (case target-kind
      ((desktop-macos) MygoDesktopMacosProfile.)
      ((desktop-linux) MygoDesktopLinuxProfile.)
      ((desktop-windows) MygoDesktopWindowsProfile.)
      ((mobile-ios) MygoIosPersonalProfile.)
      ((mobile-android) MygoAndroidPersonalProfile.)
      ((remote-headless) MygoRemoteHeadlessProfile.)
      (else (error "unknown MyGo target kind" target-kind)))))

(def (marlin-mygo-slot-declarations-for target-kind)
  (.get (marlin-mygo-profile-for target-kind) slots))

;;; Return the full ordered list of declarative UI modules mounted on target-kind
;;; (shared cross-platform core modules followed by target-specific chrome modules).
(def (marlin-mygo-modules-for target-kind)
  (let ((profile (marlin-mygo-profile-for target-kind)))
    (append (.get profile shared-modules)
            (.get profile target-modules))))

;;; Enforce BOTH target runtime allowlist (e.g. App Store declarative-only on iOS)
;;; AND slot trust boundary (approval.gate never mounts untrusted plugins).
(def (marlin-mygo-admit-plugin-slot? target-kind slot-id runtime trusted?)
  (let* ((profile (marlin-mygo-profile-for target-kind))
         (runtime-ok (and (memq runtime (.get profile allowed-runtimes)) #t))
         (slot (find (lambda (decl) (eq? (.get decl slot-id) slot-id))
                     (.get profile slots))))
    (and runtime-ok
         slot
         (or trusted?
             (and (not (memq slot-id mygo-restricted-slots))
                  (.get slot allow-untrusted-plugin?)))
         #t)))

;;; Desktop multi-pane semantic tree: Targets sidebar + Main slot + Inspector slot + Modeline.
(def (desktop-frame-tree modeline-value tab-bar)
  (ui-rows
    (list (ui-sized-with (ui-size-cells 1) (ui-region 'buffer-tabs #f tab-bar))
          (ui-sized-with (ui-size-flex 1)
            (ui-columns
              (list (ui-sized-with (ui-size-flex 3)
                      (ui-region 'transcript (ui-scroll-target 'transcript 3)
                        (ui-slot 'workbench.main)))
                    (ui-sized-with (ui-size-flex 1)
                      (ui-panel #f mygo-panel-field mygo-panel-title
                        (ui-slot 'workbench.inspector))))))
          (ui-sized-with (ui-size-cells 1) (ui-region 'modeline #f modeline-value)))))

;;; Mobile compact semantic stack: Top tab/target bar + Full-width Main slot + Status bar.
(def (mobile-frame-tree modeline-value tab-bar)
  (ui-rows
    (list (ui-sized-with (ui-size-cells 1) (ui-region 'buffer-tabs #f tab-bar))
          (ui-sized-with (ui-size-flex 1)
            (ui-region 'transcript (ui-scroll-target 'transcript 3)
              (ui-slot 'workbench.main)))
          (ui-sized-with (ui-size-cells 1) (ui-region 'modeline #f modeline-value)))))

(def (marlin-mygo-layout-policy-for layout-kind)
  (validate MygoLayoutKind layout-kind)
  (lambda (modeline-value _transcript-value tab-bar)
    (seal-object!
      (.mix (.o echo-policy: minibuffer-echo-policy
                frame-tree: (if (eq? layout-kind 'mobile-stack)
                              (mobile-frame-tree modeline-value tab-bar)
                              (desktop-frame-tree modeline-value tab-bar)))
            UiLayoutBase.))))

(def (marlin-mygo-layout-scope-for target-kind)
  (let* ((profile (marlin-mygo-profile-for target-kind))
         (layout-kind (.get profile layout-kind)))
    (validate UiLayoutScopeContract
      (.o identity: 'marlin/ui-layout
          policy-for: (marlin-mygo-layout-policy-for layout-kind)
          tab-bar-for: (.get UiCentaurTabsScope bar-for)))))

(def (marlin-mygo-make-ui-scope target-kind)
  (make-ui-scope UiComponentsScope UiModelineScope UiTranscriptScope
                 UiThemesScope (marlin-mygo-layout-scope-for target-kind)))

;;; Native Scheme vector projection of shared vs target-specific UI modules
;;; crossing C ABI / gRPC into MyGo surfaces (Desktop, iOS, Android).
(def (marlin-mygo-project-surface-manifest target-kind)
  (let ((profile (marlin-mygo-profile-for target-kind)))
    (vector 'mygo-surface-manifest
            (.get profile target-kind)
            (.get profile layout-kind)
            (list->vector (.get profile shared-modules))
            (list->vector (.get profile target-modules)))))

;;; Native Scheme vector projection crossing C ABI into Rust (zero text serialization).
(def (marlin-mygo-project-overlay target-kind)
  (let ((profile (marlin-mygo-profile-for target-kind)))
    (vector 'mygo-profile-overlay
            (.get profile target-kind)
            (.get profile layout-kind)
            (list->vector (.get profile allowed-runtimes))
            (list->vector (.get profile device-protocols))
            (list->vector
              (map (lambda (slot)
                     (vector (.get slot slot-id)
                             (.get slot cardinality)
                             (.get slot fallback-renderer)
                             (.get slot allow-untrusted-plugin?)))
                   (.get profile slots))))))
