;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Declarative POO prototypes for Organization Architecture (`Role -> Agent / SoD`)
;;; and GitOps Artifacts topology (`Project -> Domain/Worktree -> Collection -> Kind`).

(import (only-in :clan/poo/object .def .o))
(export ArtifactRole.
        ArtifactAgent.
        ArtifactOrganization.
        ArtifactKind.
        ArtifactDomain.
        ArtifactCollection.
        ArtifactCatalog.
        ArtifactPrototypes
        DefaultArtifactRoles
        DefaultArtifactAgents
        DefaultArtifactOrganization
        DefaultArtifactDomains
        DefaultArtifactCollections
        ArtifactKinds
        DefaultArtifactCatalog)

(.def ArtifactRole.
  identity: "role/base"
  title: "Base Role"
  capabilities: '()
  review-eligible?: #f)

(.def ArtifactAgent.
  identity: "agent/base"
  role: 'author
  parent: #f
  domains: '(shared worktree verification))

(.def ArtifactOrganization.
  identity: "marlin-org"
  roles: (.o)
  agents: (.o)
  sod-pairs: '((author . verifier) (operator . verifier)))

(.def ArtifactKind.
  kind: 'generic
  media: 'blob
  collection: 'patches
  required-capability: 'record-patch
  review-required?: #f
  contract-id: "context.artifact.entry.v1")

(.def ArtifactDomain.
  identity: 'shared
  path: "shared"
  worktree-scope: 'project-shared)

(.def ArtifactCollection.
  identity: 'patches
  domain: 'worktree
  directory: "worktrees/patches"
  index-file: "patches.org"
  contract-id: "context.gitops.artifacts.collection.v1"
  allowed-kinds: '())

(.def DefaultArtifactRoles
  coordinator:
  (.o (:: @ ArtifactRole.)
      identity: "role/coordinator"
      title: "Coordinator"
      capabilities: '(define-topology delegate-work review-org)
      review-eligible?: #t)
  architect:
  (.o (:: @ ArtifactRole.)
      identity: "role/architect"
      title: "Architect"
      capabilities: '(define-topology author-org review-org)
      review-eligible?: #t)
  author:
  (.o (:: @ ArtifactRole.)
      identity: "role/author"
      title: "Author"
      capabilities: '(author-org record-patch)
      review-eligible?: #f)
  verifier:
  (.o (:: @ ArtifactRole.)
      identity: "role/verifier"
      title: "Verifier"
      capabilities: '(record-receipt review-org verify-proof)
      review-eligible?: #t)
  operator:
  (.o (:: @ ArtifactRole.)
      identity: "role/operator"
      title: "Host Operator"
      capabilities: '(define-topology author-org record-patch record-receipt)
      review-eligible?: #f))

(.def DefaultArtifactAgents
  lead:
  (.o (:: @ ArtifactAgent.)
      identity: "agent/coordinator-00"
      role: 'coordinator
      parent: #f
      domains: '(shared worktree verification))
  architect:
  (.o (:: @ ArtifactAgent.)
      identity: "agent/architect-01"
      role: 'architect
      parent: 'lead
      domains: '(shared worktree))
  worker:
  (.o (:: @ ArtifactAgent.)
      identity: "agent/worker-01"
      role: 'author
      parent: 'architect
      domains: '(worktree))
  verifier:
  (.o (:: @ ArtifactAgent.)
      identity: "agent/verifier-01"
      role: 'verifier
      parent: 'lead
      domains: '(shared worktree verification))
  host-operator:
  (.o (:: @ ArtifactAgent.)
      identity: "agent/host-operator"
      role: 'operator
      parent: 'lead
      domains: '(shared worktree verification)))

(.def (DefaultArtifactOrganization @ ArtifactOrganization.)
  identity: "marlin-engineering-org"
  roles: DefaultArtifactRoles
  agents: DefaultArtifactAgents)

(.def DefaultArtifactDomains
  shared:
  (.o (:: @ ArtifactDomain.)
      identity: 'shared
      path: "shared"
      worktree-scope: 'project-shared)
  worktree:
  (.o (:: @ ArtifactDomain.)
      identity: 'worktree
      path: "worktrees"
      worktree-scope: 'worktree-bound)
  verification:
  (.o (:: @ ArtifactDomain.)
      identity: 'verification
      path: "verification"
      worktree-scope: 'verification-bound))

(.def DefaultArtifactCollections
  topology:
  (.o (:: @ ArtifactCollection.)
      identity: 'topology
      domain: 'shared
      directory: "shared/topology"
      index-file: "topology.org"
      contract-id: "context.gitops.topology.v1"
      allowed-kinds: '(topology-org))
  evidence:
  (.o (:: @ ArtifactCollection.)
      identity: 'evidence
      domain: 'shared
      directory: "shared/evidence"
      index-file: "evidence.org"
      contract-id: "context.evidence-source.v1"
      allowed-kinds: '(evidence-org))
  work:
  (.o (:: @ ArtifactCollection.)
      identity: 'work
      domain: 'worktree
      directory: "worktrees/work"
      index-file: "work.org"
      contract-id: "context.governance.work.v1"
      allowed-kinds: '(governed-work-org))
  patches:
  (.o (:: @ ArtifactCollection.)
      identity: 'patches
      domain: 'worktree
      directory: "worktrees/patches"
      index-file: "patches.org"
      contract-id: "context.gitops.artifacts.collection.v1"
      allowed-kinds: '(git-patch))
  receipts:
  (.o (:: @ ArtifactCollection.)
      identity: 'receipts
      domain: 'verification
      directory: "verification/receipts"
      index-file: "receipts.org"
      contract-id: "context.gitops.artifacts.collection.v1"
      allowed-kinds: '(test-receipt)))

(.def ArtifactKinds
  git-patch:
  (.o (:: @ ArtifactKind.)
      kind: 'git-patch
      media: 'blob
      collection: 'patches
      required-capability: 'record-patch
      review-required?: #f
      contract-id: "context.artifact.git-patch.v1")
  test-receipt:
  (.o (:: @ ArtifactKind.)
      kind: 'test-receipt
      media: 'blob
      collection: 'receipts
      required-capability: 'record-receipt
      review-required?: #f
      contract-id: "context.artifact.test-receipt.v1")
  topology-org:
  (.o (:: @ ArtifactKind.)
      kind: 'topology-org
      media: 'org
      collection: 'topology
      required-capability: 'define-topology
      review-required?: #t
      contract-id: "context.gitops.topology.v1")
  governed-work-org:
  (.o (:: @ ArtifactKind.)
      kind: 'governed-work-org
      media: 'org
      collection: 'work
      required-capability: 'author-org
      review-required?: #t
      contract-id: "context.governance.work.v1")
  evidence-org:
  (.o (:: @ ArtifactKind.)
      kind: 'evidence-org
      media: 'org
      collection: 'evidence
      required-capability: 'author-org
      review-required?: #f
      contract-id: "context.evidence-source.v1"))

(.def ArtifactCatalog.
  identity: "marlin-gitops-artifacts"
  project: "marlin-agent-core"
  contract-id: "context.gitops.artifacts.index.v1"
  contract-revision: "1"
  organization: DefaultArtifactOrganization
  domains: DefaultArtifactDomains
  collections: DefaultArtifactCollections
  kinds: ArtifactKinds)

(.def (DefaultArtifactCatalog @ ArtifactCatalog.))

(.def ArtifactPrototypes
  role: ArtifactRole.
  agent: ArtifactAgent.
  organization: ArtifactOrganization.
  kind: ArtifactKind.
  domain: ArtifactDomain.
  collection: ArtifactCollection.
  catalog: ArtifactCatalog.)
