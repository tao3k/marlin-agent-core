;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/type Fun Record Symbol Top Type.))
(export ArtifactsContract)

(def ArtifactContract (Record kind: [Symbol]))
(def ArtifactKindsContract
  (Record git-patch: [ArtifactContract] test-receipt: [ArtifactContract]))

(def ArtifactProjection (.cc Type. .element?: vector?))

(def ArtifactsContract
  (.cc (Record kinds: [ArtifactKindsContract]
               exchange: [(Fun ArtifactProjection <- Top)])
       identity: 'marlin/artifacts))
