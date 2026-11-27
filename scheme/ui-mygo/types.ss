;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure Scheme POO contracts for Marlin MyGo surface overlays and slot policy.
;;; Strictly host-neutral: zero MyGo widget calls, zero text serialization.
(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/type Bool Enum Exactly Fun List Object Record Symbol Type.))
(export MygoTargetKind MygoLayoutKind MygoPanelRuntime MygoSlotCardinality
        MygoVector MygoSlotDeclarationContract
        MygoProfileOverlayContract UiMygoScopeContract)

(def MygoVector (.cc Type. '.element? vector?))

(def MygoTargetKind
  (Enum desktop-macos desktop-linux desktop-windows
        mobile-ios mobile-android remote-headless))

(def MygoLayoutKind
  (Enum desktop-split mobile-stack))

(def MygoPanelRuntime
  (Enum declarative webview-untrusted wasm-ui-ir mygo-js-wasm))

(def MygoSlotCardinality
  (Enum single list keyed))

(def MygoSlotDeclarationContract
  (Record slot-id: [Symbol]
          cardinality: [MygoSlotCardinality]
          fallback-renderer: [Symbol]
          allow-untrusted-plugin?: [Bool]))

(def MygoProfileOverlayContract
  (Record target-kind: [MygoTargetKind]
          layout-kind: [MygoLayoutKind]
          allowed-runtimes: [(List MygoPanelRuntime)]
          device-protocols: [(List Symbol)]
          shared-modules: [(List Symbol)]
          target-modules: [(List Symbol)]
          slots: [(List MygoSlotDeclarationContract)]))

(def UiMygoScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-mygo)]
               profile-for: [(Fun MygoProfileOverlayContract <- MygoTargetKind)]
               layout-scope-for: [(Fun Object <- MygoTargetKind)]
               slot-declarations-for: [(Fun (List MygoSlotDeclarationContract) <- MygoTargetKind)]
               modules-for: [(Fun (List Symbol) <- MygoTargetKind)]
               admit-plugin-slot?: [(Fun Bool <- MygoTargetKind Symbol MygoPanelRuntime Bool)]
               project-profile-overlay: [(Fun MygoVector <- MygoTargetKind)]
               project-surface-manifest: [(Fun MygoVector <- MygoTargetKind)]
               make-mygo-ui-scope: [(Fun Object <- MygoTargetKind)])
       identity: 'marlin/ui-mygo))
