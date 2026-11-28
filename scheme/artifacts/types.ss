;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc object?)
        (only-in :clan/poo/type Bool Fun List Record String Symbol Top Type.))
(export ArtifactRoleContract
        ArtifactAgentContract
        ArtifactOrganizationContract
        ArtifactKindContract
        ArtifactKindsContract
        ArtifactDomainContract
        ArtifactCollectionContract
        ArtifactWorkspaceLayoutContract
        ArtifactCatalogContract
        ArtifactProjection
        ArtifactsContract)

(def PooObjectType (.cc Type. .element?: object?))
(def SymbolListType (List Symbol))

(def ArtifactRoleContract
  (Record identity: [String]
          title: [String]
          capabilities: [SymbolListType]
          review-eligible?: [Bool]))

(def ArtifactAgentContract
  (Record identity: [String]
          role: [Symbol]
          parent: [Top]
          domains: [SymbolListType]))

(def ArtifactOrganizationContract
  (Record identity: [String]
          roles: [PooObjectType]
          agents: [PooObjectType]
          sod-pairs: [Top]))

(def ArtifactKindContract
  (Record kind: [Symbol]
          media: [Symbol]
          collection: [Symbol]
          required-capability: [Symbol]
          review-required?: [Bool]
          contract-id: [String]))

(def ArtifactKindsContract
  (Record git-patch: [ArtifactKindContract]
          test-receipt: [ArtifactKindContract]))

(def ArtifactDomainContract
  (Record identity: [Symbol]
          path: [String]
          worktree-scope: [Symbol]))

(def ArtifactCollectionContract
  (Record identity: [Symbol]
          domain: [Symbol]
          directory: [String]
          index-file: [String]
          contract-id: [String]
          allowed-kinds: [SymbolListType]))

(def ArtifactWorkspaceLayoutContract
  (Record session-db: [String]
          context-db: [String]
          search-dir: [String]
          process-ledger-suffix: [String]
          project-prefix: [String]
          session-prefix: [String]
          default-session: [String]
          default-agent-key: [Symbol]))

(def ArtifactCatalogContract
  (Record identity: [String]
          project: [String]
          contract-id: [String]
          contract-revision: [String]
          organization: [ArtifactOrganizationContract]
          domains: [PooObjectType]
          collections: [PooObjectType]
          kinds: [ArtifactKindsContract]
          workspace-layout: [ArtifactWorkspaceLayoutContract]))

(def ArtifactProjection (.cc Type. .element?: vector?))

(def ArtifactsContract
  (.cc (Record prototypes: [PooObjectType]
               kinds: [ArtifactKindsContract]
               organization: [ArtifactOrganizationContract]
               catalog: [ArtifactCatalogContract]
               admit-record: [(Fun PooObjectType <- ArtifactCatalogContract PooObjectType)]
               project-context-org: [(Fun String <- ArtifactCatalogContract Top)]
               inspect-context-org: [(Fun PooObjectType <- ArtifactCatalogContract String)]
               plan-artifact-directory: [(Fun PooObjectType <- ArtifactCatalogContract Top)]
               materialize-artifact-directory!: [(Fun PooObjectType <- ArtifactCatalogContract Top String)]
               validate-artifact-directory: [(Fun PooObjectType <- String)]
               project-workspace-layout: [(Fun ArtifactProjection <- ArtifactCatalogContract)]
               exchange: [(Fun ArtifactProjection <- Top)])
       identity: 'marlin/artifacts))
