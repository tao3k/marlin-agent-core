;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" ArtifactsScope)
        (only-in "./types.ss" ArtifactsContract))
(export Artifacts)

(.def Artifacts
  identity: 'artifacts
  core: (.o scope-contract: ArtifactsContract
            exports: ArtifactsScope))
