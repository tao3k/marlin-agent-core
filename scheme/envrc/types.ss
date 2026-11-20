;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/type Bool Enum Fun Record))

(export EnvrcRequest EnvrcScopeContract)

(def EnvrcIntent (Enum activate allow reload deny))
(def EnvrcPlan (Enum absent export allow-and-export reload-and-export deny))

(def EnvrcRequest
  (Record intent: [EnvrcIntent] project-envrc?: [Bool]))

(def EnvrcScopeContract
  (.cc (Record plan: [(Fun EnvrcPlan <- EnvrcRequest)])
       identity: 'marlin/envrc))
