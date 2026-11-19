;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/type Object Fun Type.)
        (only-in :clan/poo/type Exactly Record)
        (only-in "../ui-meow/types.ss" KeyBindings))
(export UiHydraScopeContract)

(def HydraEntries (.cc Type. '.element? vector?))
(def HydraCommands (.cc Type. '.element? list?))
(def UiHydraScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-hydra)]
               catalog: [Object]
               key-bindings: [KeyBindings]
               command-entries: [(Fun HydraCommands <-)]
               entries: [(Fun HydraEntries <-)])
       identity: 'marlin/ui-hydra))
