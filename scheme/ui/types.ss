;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Typed publication contract for the Scheme-owned terminal workbench.
(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/mop Bool Fun Object Type. element?)
        (only-in :clan/poo/type Exactly List Record String)
        (only-in :yuanxu/poo-clos/types ClosGenericFunction ClosGenericProtocol)
        (only-in "./workbench.ss" marlin-ui-bounded-list?))

(export UiScopeContract)

(def UiIdentity (.cc Type. '.element? symbol?))
(def UiContribution (Record identity: [UiIdentity] label: [String]))
(def UiExtensionRegion
  (Record identity: [UiIdentity] kind: [(Exactly 'extension)] label: [String] focusable: [Bool]))
(def (bounded-list-type item-type limit)
  (let (items (List item-type))
    (.cc Type. '.element?
         (lambda (value)
           (and (marlin-ui-bounded-list? value limit) (element? items value))))))
(def UiWorkbenchFacts
  (Record model: [String] session: [String] contributions: [(bounded-list-type UiContribution 8)]
          extensions: [(bounded-list-type UiExtensionRegion 16)]))
(def UiRegion (Record identity: [UiIdentity] kind: [UiIdentity]))
(def UiWorkbench
  (Record identity: [(Exactly 'workbench)] kind: [(Exactly 'workbench)]
          model: [String] session: [String] regions: [(List UiRegion)]
          focus-order: [(List UiIdentity)]))
(def UiNativeProjection (.cc Type. '.element? vector?))
(def UiMethodBundles (.cc Type. '.element?
  (lambda (value) (and (vector? value) (<= (vector-length value) 16)))))

;; Constructors also validate size and ownership; no runtime authority is conveyed.
(def UiScopeContract
  (.cc (Record workbench: [(Fun UiWorkbench <- UiWorkbenchFacts)]
               project-workbench: [(Fun UiNativeProjection <- UiWorkbench)]
               focus-event: [(Fun Object <- UiWorkbench)]
               make-dispatch: [(Fun ClosGenericFunction <- UiMethodBundles)]
               dispatch: [(Fun Object <- ClosGenericFunction Object Object)]
               components: [Object] events: [Object] event-protocol: [ClosGenericProtocol])
       identity: 'marlin/ui))
