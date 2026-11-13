;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; The Module identity is metadata, not a native ABI readiness receipt.
(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/mop Type. element?)
        (only-in :clan/poo/type Exactly Record)
        (only-in :marlin-agent-core/ui/types UiScopeContract)
        (only-in :marlin-agent-core/context/types MarlinContextModuleContract)
        (only-in :marlin-agent-core/deck-runtime/types DeckScopeContract)
        (only-in :marlin-agent-core/tooling/types ToolingScopeContract))
(export CAbiScopeContract)
;;; Check scope membership without normalizing its callable fields again.
;;; Wrapping nested Fun contracts would replace the already-linked scope.
(def UiLinkedScope (.cc Type. '.element? (lambda (value) (element? UiScopeContract value))))
(def ContextLinkedScope (.cc Type. '.element? (lambda (value) (element? MarlinContextModuleContract value))))
(def DeckLinkedScope (.cc Type. '.element? (lambda (value) (element? DeckScopeContract value))))
(def ToolingLinkedScope (.cc Type. '.element? (lambda (value) (element? ToolingScopeContract value))))
(def CAbiScopeContract
  (.cc (Record identity: [(Exactly 'marlin/c-abi)]
               ui: [UiLinkedScope] context: [ContextLinkedScope]
               deck-runtime: [DeckLinkedScope] tooling: [ToolingLinkedScope])
       identity: 'marlin/c-abi))
