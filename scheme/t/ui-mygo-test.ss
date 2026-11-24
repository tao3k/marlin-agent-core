;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Gerbil Scheme regression tests for symmetric scheme/ui-mygo Target Profile
;;; Overlays, App Store runtime policy, restricted slot guards, and UiScope composition.
(import :std/test
        (only-in :clan/poo/object .get .o)
        (only-in :clan/poo/type TypeError?)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports))

(export ui-mygo-test)

(def ui-mygo (.get marlin-agent-core.init.exports Marlin ui-mygo))

(def ui-mygo-test
  (test-suite "Marlin Scheme ui-mygo Target Profile Overlays & Slot Admission"

    (test-case "Desktop vs iOS/Android overlays inherit base slots and specialize runtimes + device protocols"
      (let ((mac ((.get ui-mygo profile-for) 'desktop-macos))
            (ios ((.get ui-mygo profile-for) 'mobile-ios))
            (android ((.get ui-mygo profile-for) 'mobile-android)))
        (check-equal? (.get mac layout-kind) 'desktop-split)
        (check-equal? (.get ios layout-kind) 'mobile-stack)
        (check-equal? (.get android layout-kind) 'mobile-stack)
        ;; Desktop allows full TS WebView + Go WASM-IR ecosystem
        (check-equal? (and (memq 'webview-untrusted (.get mac allowed-runtimes)) #t) #t)
        (check-equal? (and (memq 'wasm-ui-ir (.get mac allowed-runtimes)) #t) #t)
        ;; Mobile overlays enforce App Store / Play Store declarative-only packs
        (check-equal? (.get ios allowed-runtimes) '(declarative))
        (check-equal? (.get android allowed-runtimes) '(declarative))
        ;; Device protocol families stay target-specific via POO inheritance
        (check-equal? (and (memq 'device.ios.photos (.get ios device-protocols)) #t) #t)
        (check-equal? (and (memq 'device.ios.share-extension (.get ios device-protocols)) #t) #t)
        (check-equal? (and (memq 'device.android.content-resolver (.get android device-protocols)) #t) #t)))

    (test-case "Slot admission enforces both restricted approval.gate trust and mobile App Store runtime rules"
      (let ((admit? (.get ui-mygo admit-plugin-slot?)))
        ;; Desktop admits untrusted TS WebView & Go WASM-IR in workbench.main
        (check-equal? (admit? 'desktop-macos 'workbench.main 'webview-untrusted #f) #t)
        (check-equal? (admit? 'desktop-macos 'workbench.main 'wasm-ui-ir #f) #t)
        ;; Restricted approval.gate NEVER admits untrusted plugins on any target
        (check-equal? (admit? 'desktop-macos 'approval.gate 'webview-untrusted #f) #f)
        (check-equal? (admit? 'desktop-macos 'approval.gate 'declarative #f) #f)
        (check-equal? (admit? 'mobile-ios 'approval.gate 'declarative #f) #f)
        ;; Host-trusted host.gate renderer IS admitted in approval.gate
        (check-equal? (admit? 'desktop-macos 'approval.gate 'declarative #t) #t)
        ;; iOS admits declarative packs in workbench.main, but rejects executable webview/wasm plugins
        (check-equal? (admit? 'mobile-ios 'workbench.main 'declarative #f) #t)
        (check-equal? (admit? 'mobile-ios 'workbench.main 'webview-untrusted #f) #f)
        (check-equal? (admit? 'mobile-ios 'workbench.main 'wasm-ui-ir #f) #f)
        ;; Unknown target kind raises POO TypeError
        (check-exception ((.get ui-mygo profile-for) 'smart-fridge) TypeError?)))

    (test-case "MyGo layouts compose seamlessly into standard UiScope and project Scheme vectors"
      (let* ((mac-ui ((.get ui-mygo make-mygo-ui-scope) 'desktop-macos))
             (ios-ui ((.get ui-mygo make-mygo-ui-scope) 'mobile-ios))
             (facts (.o model: "claude-sonnet" session: "mygo-dev" contributions: '() extensions: '()))
             (mac-wb ((.get mac-ui workbench) facts))
             (ios-wb ((.get ios-ui workbench) facts))
             (mac-proj ((.get mac-ui project-workbench) mac-wb))
             (ios-proj ((.get ios-ui project-workbench) ios-wb))
             (overlay-vec ((.get ui-mygo project-profile-overlay) 'mobile-ios)))
        ;; Both desktop & mobile MyGo UiScopes project valid 5-field workbench vectors
        (check-equal? (vector-length mac-proj) 5)
        (check-equal? (vector-length ios-proj) 5)
        ;; Overlay vector carries exact target, layout, runtimes, protocols, and 4 slots
        (check-equal? (vector-ref overlay-vec 0) 'mygo-profile-overlay)
        (check-equal? (vector-ref overlay-vec 1) 'mobile-ios)
        (check-equal? (vector-ref overlay-vec 2) 'mobile-stack)
        (check-equal? (vector-ref overlay-vec 3) '#(declarative))
        (check-equal? (vector-length (vector-ref overlay-vec 5)) 4)
        ;; Fourth slot is approval.gate owned by host.gate with allow-untrusted = #f
        (check-equal? (vector-ref (vector-ref (vector-ref overlay-vec 5) 3) 0) 'approval.gate)
        (check-equal? (vector-ref (vector-ref (vector-ref overlay-vec 5) 3) 2) 'host.gate)
        (check-equal? (vector-ref (vector-ref (vector-ref overlay-vec 5) 3) 3) #f)))))
