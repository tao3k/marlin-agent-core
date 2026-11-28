;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .o)
        (only-in :clan/poo/type validate)
        "./types.ss" "./objects.ss" "./funcs.ss")
(export ArtifactsScope)

(def ArtifactsScope
  (validate ArtifactsContract
    (.o prototypes: ArtifactPrototypes
        kinds: ArtifactKinds
        organization: DefaultArtifactOrganization
        catalog: DefaultArtifactCatalog
        admit-record: artifacts-admit-record
        project-context-org: artifacts-project-context-org
        inspect-context-org: artifacts-inspect-context-org
        plan-artifact-directory: artifacts-plan-artifact-directory
        materialize-artifact-directory!: artifacts-materialize-artifact-directory!
        validate-artifact-directory: artifacts-validate-artifact-directory
        project-workspace-layout: artifacts-project-workspace-layout
        exchange: artifacts-exchange)))
