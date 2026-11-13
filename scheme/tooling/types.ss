;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/mop Fun Type.)
        (only-in :clan/poo/type Exactly Record))
(export ToolingScopeContract)

(def NativeToolDeclarations (.cc Type. '.element? vector?))
(def ToolingScopeContract
  (.cc (Record identity: [(Exactly 'marlin/tooling)]
               project: [(Fun NativeToolDeclarations <-)])
       identity: 'marlin/tooling))
