;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; POO type contracts for Marlin's Gerbil Scheme Emacs Primitives
;;; (`scheme/ui-emacs`): Bookmarks, Kill Ring, Better-Jumper Navigation Stack,
;;; and Consult Virtual Source & Imenu Policies.
(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/type Bool Exactly List Record String Symbol Type.))
(export BookmarkEntryContract
        KillRingContract
        JumpEntryContract
        JumpListContract
        ConsultSourceContract
        EmacsSessionContract
        UiEmacsScopeContract)

(def IntContract
  (.cc Type. '.element? exact-integer?))

(def ProcContract
  (.cc Type. '.element? procedure?))

(def BookmarkEntryContract
  (Record
    name: [String]
    path: [String]
    line: [IntContract]
    context: [String]
    project-root: [String]))

(def KillRingContract
  (Record
    entries: [(List String)]
    max-size: [IntContract]
    yank-index: [IntContract]))

(def JumpEntryContract
  (Record
    buffer-id: [String]
    path: [String]
    line: [IntContract]
    label: [String]))

(def JumpListContract
  (Record
    entries: [(List JumpEntryContract)]
    cursor: [IntContract]
    max-size: [IntContract]))

(def ConsultSourceContract
  (Record
    id: [Symbol]
    label: [String]
    narrow-key: [String]
    engine: [Symbol]
    priority: [IntContract]
    async?: [Bool]))

(def EmacsSessionContract
  (Record
    bookmarks: [(List BookmarkEntryContract)]
    kill-ring: [KillRingContract]
    jump-list: [JumpListContract]
    consult-sources: [(List ConsultSourceContract)]))

(def UiEmacsScopeContract
  (.cc (Record
         identity: [(Exactly 'marlin/ui-emacs)]
         default-consult-sources: [(List ConsultSourceContract)]
         make-session: [ProcContract]
         bookmark-set: [ProcContract]
         bookmark-jump: [ProcContract]
         bookmark-delete: [ProcContract]
         bookmark-list: [ProcContract]
         kill-push: [ProcContract]
         yank-current: [ProcContract]
         yank-pop: [ProcContract]
         jump-push: [ProcContract]
         jump-back: [ProcContract]
         jump-forward: [ProcContract]
         project-consult-sources: [ProcContract])
       identity: 'marlin/ui-emacs))
