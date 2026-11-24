;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; POO prototypes for cross-platform MyGo Target Profile Overlays.
;;; Demonstrates M+N leverage: every surface inherits the same slot trust
;;; invariants from MygoBaseProfile. and overrides only its target delta.
(import (only-in :clan/poo/object .def .o seal-object!)
        (only-in "./types.ss" MygoSlotDeclarationContract)
        (only-in :clan/poo/type validate))
(export mygo-restricted-slots
        MygoBaseSlots
        MygoBaseProfile.
        MygoDesktopMacosProfile.
        MygoDesktopLinuxProfile.
        MygoDesktopWindowsProfile.
        MygoIosPersonalProfile.
        MygoAndroidPersonalProfile.
        MygoRemoteHeadlessProfile.)

;;; Restricted slots can never host untrusted Marketplace plugins on any surface.
(def mygo-restricted-slots '(approval.gate host.auth host.secrets))

(def (make-slot slot-id cardinality fallback-renderer allow-untrusted-plugin?)
  (seal-object!
    (validate MygoSlotDeclarationContract
      (.o (slot-id := slot-id)
          (cardinality := cardinality)
          (fallback-renderer := fallback-renderer)
          (allow-untrusted-plugin? := allow-untrusted-plugin?)))))

(def MygoBaseSlots
  (list (make-slot 'workbench.main 'single 'host.main #t)
        (make-slot 'workbench.inspector 'single 'host.inspector #t)
        (make-slot 'workbench.tool-card 'keyed 'host.tool-card #t)
        (make-slot 'approval.gate 'single 'host.gate #f)))

(.def MygoBaseProfile.
  target-kind: 'remote-headless
  layout-kind: 'desktop-split
  allowed-runtimes: '(declarative)
  device-protocols: '()
  slots: MygoBaseSlots)

(.def (MygoDesktopMacosProfile. @ MygoBaseProfile.)
  target-kind: 'desktop-macos
  layout-kind: 'desktop-split
  allowed-runtimes: '(declarative webview-untrusted wasm-ui-ir mygo-js-wasm)
  device-protocols: '(device.desktop.clipboard device.desktop.notifications device.desktop.files-picker))

(.def (MygoDesktopLinuxProfile. @ MygoDesktopMacosProfile.)
  target-kind: 'desktop-linux)

(.def (MygoDesktopWindowsProfile. @ MygoDesktopMacosProfile.)
  target-kind: 'desktop-windows)

;;; iOS Personal Assistant Overlay: App Store compliant (declarative packs only)
;;; with compiled platform.ios device protocol providers + remote gRPC mirroring.
(.def (MygoIosPersonalProfile. @ MygoBaseProfile.)
  target-kind: 'mobile-ios
  layout-kind: 'mobile-stack
  allowed-runtimes: '(declarative)
  device-protocols: '(device.ios.photos device.ios.eventkit device.ios.contacts device.ios.voice device.ios.share-extension))

;;; Android Personal Assistant Overlay: Play Store safe declarative packs + platform.android device protocols.
(.def (MygoAndroidPersonalProfile. @ MygoBaseProfile.)
  target-kind: 'mobile-android
  layout-kind: 'mobile-stack
  allowed-runtimes: '(declarative)
  device-protocols: '(device.android.content-resolver device.android.notifications device.android.intents))

(.def (MygoRemoteHeadlessProfile. @ MygoBaseProfile.)
  target-kind: 'remote-headless
  layout-kind: 'desktop-split
  allowed-runtimes: '(declarative)
  device-protocols: '())
