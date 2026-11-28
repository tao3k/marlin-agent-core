;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; POO type contracts for Marlin's Unified Scheme Bridge Hub (`scheme/ui-bridge`):
;;; one modular Scheme dispatch registry hosting Org Mode (`orgize`), Markdown,
;;; and future Emacs/Scheme packages behind one native C-ABI vector boundary.
(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/type Exactly List Record String Type.))
(export BridgeModuleContract
        UiBridgeScopeContract)

(def ProcContract
  (.cc Type. '.element? procedure?))

(def BridgeModuleContract
  (Record
    namespace: [String]
    owner-id: [String]
    description: [String]
    handle-query: [ProcContract]))

(def UiBridgeScopeContract
  (.cc (Record
         identity: [(Exactly 'marlin/ui-bridge)]
         default-modules: [(List BridgeModuleContract)]
         list-namespaces: [ProcContract]
         register-module: [ProcContract]
         dispatch: [ProcContract])
       identity: 'marlin/ui-bridge))
