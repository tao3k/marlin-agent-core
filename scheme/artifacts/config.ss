;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .o)
        (only-in :clan/poo/type validate)
        "./types.ss" "./objects.ss" "./funcs.ss")
(export ArtifactsScope)

(def ArtifactsScope
  (validate ArtifactsContract
    (.o kinds: ArtifactKinds exchange: artifacts-exchange)))
