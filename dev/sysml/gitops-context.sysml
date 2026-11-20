// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

package MarlinGitOpsContext {
    private import ScalarValues::*;

    doc /* Review model. The Lab and MRR/Data audit and proposed decisions are recorded in
           40.182-gitops-context-discovery.org. Allocations describe intended
           responsibilities and require downstream qualification. */

    part actual {
        part labGitOps {
            doc /* Lab publishes Org-bound worktree create policy and receipt
                   interpretation. Its consumer owns physical Git observation
                   and effects. */
        }
        part labContext {
            doc /* The Worktree Feature resolves Task and Org leaves. The
                   bounded GQL Provider evaluates registered source cuts.
                   Task publication rechecks the selected cuts and grants. */
        }
        part labOrgFixtureEvidence {
            doc /* The pinned Org descendants suite qualifies base and
                   WorkTrunk Contract fixtures and declared relative file
                   links inside the selected Worktree namespace. Attachment
                   and undeclared file links remain rendered rows. The suite
                   uses supplied Module selections and source bytes.
                   Physical Repo binding and cross-Repo traversal require
                   separate qualification. */
        }
        part labPublicationFixture {
            doc /* The pinned Task publication fixture selected WorkTrunk
                   on a synthetic GitOps core with only a component slot.
                   GitOps core publishes working-copy-policy. WorkTrunk
                   refines that object. Supplying the exported core policy
                   in a test-only replay passes the full suite. */
        }
        part crossRepoPublicationFixture {
            doc /* PR 34 design blobs at c2c6bc8 seed both Org projections.
                   Marlin's Rust Worktrunk driver creates both Worktrees.
                   The fixture consumes the driver receipts and observes the
                   Git common directories, refs, input cuts and object IDs.
                   The Scheme reader checks both Lab display grants before
                   invoking the Git blob reader. Either missing or revoked
                   grant yields zero Git child events and zero pack accesses
                   for both repositories. With both grants, Git reads the
                   two independently verified blob offsets. Revocation after
                   both batch children deliver their first payload character
                   discards both completions. No new Task result or Query
                   history appears, and the warm cache retains generation one.
                   Renewed authority causes a fresh Git read and advances both
                   source lineages. The reader exercises a serialized caller. */
        }
        part marlinContextBridge {
            doc /* Marlin composes Lab core exports and exposes typed Org
                   source and Contract operations. */
        }
        part marlinArtifactExperiment {
            doc /* The workspace experiment materializes a single Org view
                   from the Lab durable journal. It exercises record/restore,
                   with no cross-repository discovery or TUI integration. */
        }
        dependency invokesOrgBoundary from marlinArtifactExperiment to marlinContextBridge;
        dependency composesLabContext from marlinContextBridge to labContext;
        dependency qualifiesOrgSelection from labOrgFixtureEvidence to labContext;
        dependency qualifiesFixtureComposition from labPublicationFixture to labContext;
        dependency fixtureRequiresCorePolicy from labPublicationFixture to labGitOps;
        dependency qualifiesExplicitOrigin from crossRepoPublicationFixture to labContext;
    }

    attribute def RepositoryBinding {
        attribute remoteLocator : String [0..1];
        attribute localInstance : String;
        attribute commonGitDirectory : String;
        attribute ownerDirectoryIdentity : String;
        doc /* The URL host and repository path route discovery; a local Git
               instance binds an observation. A Lab Project may span repos. */
    }
    enum def SourceLens {
        enum head;
        enum index;
        enum working;
        enum selectedUntracked;
    }
    attribute def CapturedInput {
        attribute path : String;
        attribute lens : SourceLens;
        attribute contentIdentity : String;
        doc /* The source owner retains the identified bytes or returns an
               explicit unavailable observation for the requested input. */
    }
    attribute def GitObservation {
        attribute repository : RepositoryBinding;
        attribute worktree : String;
        attribute branchRef : String [0..1];
        attribute headCommit : String;
        attribute capturedInput : CapturedInput [0..*];
        doc /* A detached HEAD omits branchRef. */
    }
    attribute def OrgSourceCut {
        attribute sourceIdentity : String;
        attribute generation : Integer;
        attribute sourceDigest : String;
        attribute contractIdentity : String;
        attribute contractRevision : String;
        attribute origin : GitObservation;
    }
    attribute def AnchoredContextReport {
        attribute worktree : GitObservation;
        attribute index : OrgSourceCut;
        attribute worktreeTopology : OrgSourceCut;
        attribute branchTopology : OrgSourceCut [0..1];
        doc /* A top-level Report anchors the checkout's index and topology.
               A selected branch has its own topology. A detached checkout
               has no branch topology. The root CONTRACT_ID distinguishes
               each document's profile. The captured Git input cut precedes
               the publication object that stores the resulting Org bytes. */
    }
    attribute def SourceLocator {
        attribute cut : OrgSourceCut;
        attribute owner : String;
        attribute ownerLocator : String;
        doc /* Resolve an owning Git object/path or an admitted durable capture.
               Qualified anchors refer to exact source cuts. */
    }
    attribute def QualifiedOrgAnchor {
        attribute source : OrgSourceCut;
        attribute localId : String;
    }
    attribute def RefRetirement {
        attribute repository : RepositoryBinding;
        attribute sourceRef : String;
        attribute sourceTip : String;
        attribute reason : String;
        attribute targetCommit : String [0..1];
        attribute evidence : String;
        doc /* Merge evidence binds the exact source ref and tip. A deletion
               has its own reason and can omit a target commit. */
    }
    attribute def RetentionHold {
        attribute source : SourceLocator;
        attribute producerGeneration : Integer;
        attribute referenceOwner : String;
        doc /* A visible reference has an owner hold before publication.
               Use a private Git retention ref or a producer-owned capture. */
    }

    part <'MAR-GCTX-A01'> target {
        part gitOps {
            doc /* Marlin composes Lab GitOps policy. Explicit enrollment
                   binds an instance to common Git directory and owner
                   identity. Remote routing and current branch are facts
                   with independent lifetimes. */
        }
        part resourceOwner {
            doc /* Git or the producer's storage owns Org and referenced
                   artifact bytes. GitOps holds retained commits with private
                   refs. Capture producers own verified content and any
                   qualified deletion capability. Lab provides policy. */
        }
        part discoveryIndex {
            doc /* Marlin schema on the pinned MRR Data Turso driver.
                   Affected memberships and owner cursor commit together.
                   Scope distinguishes home and DEV catalogs. */
        }
        part labAdmission {
            doc /* Compose the Lab Worktree Feature. Orgize parses source
                   properties, links and contract scope. Lab resolves Org
                   cuts and admits bounded GQL results. The Lab Context
                   library owns the basic Index and Topology templates and
                   the Scheme operations for parsing them. Marlin's Scheme
                   composes and overrides the projections using typed GitOps
                   observations from the WorkTrunk provider. */
        }
        part mrrQueryExtension {
            doc /* Optional qualified graph execution over admitted relations.
                   MRR owns catalog/result admission; MRR Data executes only
                   the verified physical snapshot and profile. */
        }
        part tui {
            doc /* After the first frame, a bounded restore request uses the
                   launch Worktree facts and current display scope. */
        }
        dependency readWorktree from tui to gitOps;
        dependency discoverCandidates from tui to discoveryIndex;
        dependency projectCandidateLocations from discoveryIndex to resourceOwner;
        dependency proveMembership from discoveryIndex to gitOps;
        dependency requestContext from tui to labAdmission;
        dependency readAuthorizedSource from labAdmission to resourceOwner;
        dependency graphQueriesRequireAdmission from mrrQueryExtension to labAdmission;
        dependency retainCommit from resourceOwner to gitOps;
    }

    requirement <'MAR-GCTX-R01'> sourceIdentity {
        doc /* Distinguish Lab Project, remote locator, local repository,
               Worktree, optional branch ref, source identity and qualified
               Org anchor. Bind explicit remote, origin or sole eligible
               network remote in that order. Route local repositories by
               instance. Refuse ambiguous remote or registration binding.
               Bind results to exact selected observations. */
    }
    requirement <'MAR-GCTX-R02'> ownerAndDiscovery {
        doc /* Keep Org and artifact bytes under their producer. Derive the
               locator index from owner evidence. DEV=1 selects the isolated
               repository development scope; ordinary mode uses the user's
               marlin-ai root. Publish atomic index deltas with owner cursors.
               Cursor gaps report incomplete coverage until reconciled. */
    }
    requirement <'MAR-GCTX-R03'> contextAdmission {
        doc /* Narrow by declared repository relationships and consumer scope.
               Apply visibility to locator metadata and check a display grant
               before protected source I/O. Revalidate source cut, Query result,
               Contract and authority at publication. Cached or historical
               evidence has its original lineage. */
    }
    requirement <'MAR-GCTX-R04'> historyAndRetention {
        doc /* Retire active membership with matching merge, ref deletion or
               replacement evidence and preserve the reason. Require ancestry
               or an admitted exact-tip rewrite merge receipt. Acquire the
               resource hold before publishing a reference. Keep original
               bytes by default. Expiry, absence of holds, producer generation
               and known coverage must coincide at fenced disposal. */
    }
    requirement <'MAR-GCTX-R05'> incrementalQualification {
        doc /* A qualified warm restore reads index metadata and matching
               source windows. One source change updates its projection and
               dependent entries. Measure lookup, update and rebuild costs
               independently with equivalent admitted results. */
    }
    requirement <'MAR-GCTX-R06'> startupRestore {
        doc /* Render the first frame independently. Submit a bounded,
               cancellable read keyed by the Git observation and display scope.
               Report unavailable, stale and incomplete coverage explicitly.
               A replaced request cannot publish a late view. */
    }
    requirement <'MAR-GCTX-R07'> orgContractIntegration {
        doc /* Bind the declared Org properties to the observed Worktree,
               source and Contract. Qualify each anchor by source and
               generation. Reuse the declared relative-file and origin
               resolution in the owning Worktree Module. Keep unsupported
               link forms as content. Qualify a cross-Repo hop with
               independent Git and scope evidence before publishing it.
               Preserve Org bytes and treat ordinary blocks as data.
               The repository index and topology generate property values
               from typed declarations and observed owner facts. Use distinct
               Worktree and Branch Contracts for the anchored topologies.
               Pin the source and target publication, Org blob and generation.
               Check both current display grants before opening either
               protected object. Invalidate a prepared read if either grant
               changes before the open.
               After the read returns, recheck both grants before admitting
               either payload to the source catalog, its recovery history or
               a caller-owned cache. Preserve prior valid cached generations
               and require a new read after authority is restored.
               Recheck both display grants and current cuts at publication. */
    }
    requirement <'MAR-GCTX-R09'> gitOpsPolicyComposition {
        doc /* Working-copy policy belongs to the GitOps core scope.
               WorkTrunk refines an admitted core policy and preserves its
               base provider. Task publication belongs to Context and
               consumes the selected GitOps dependency. */
    }
    requirement <'MAR-GCTX-R08'> nativeQueryQualification {
        doc /* Qualify an exact MRR/Data producer pair. Check original GQL,
               relation/entity catalog, snapshot, profile, result admission
               and source lineage before adopting native result rows. */
    }

    allocation resolveGitIdentity allocate sourceIdentity to target.gitOps;
    allocation deriveLocators allocate ownerAndDiscovery to target.discoveryIndex;
    allocation authorizeAndQuery allocate contextAdmission to target.labAdmission;
    allocation retainSource allocate historyAndRetention to target.resourceOwner;
    allocation measureDiscovery allocate incrementalQualification to target.discoveryIndex;
    allocation restoreView allocate startupRestore to target.tui;
    allocation bindOrgContract allocate orgContractIntegration to target.labAdmission;
    allocation qualifyNativeQuery allocate nativeQueryQualification to target.mrrQueryExtension;
    allocation composeWorkingCopyPolicy allocate gitOpsPolicyComposition to target.gitOps;

    attribute def CutAdmitted;
    attribute def CutSuperseded;
    attribute def KeepForReview;
    attribute def DisposalEligible;
    attribute def DisposalCommitted;

    state def <'MAR-GCTX-L01'> LocatorLifecycle {
        doc /* State for one exact cut. A replacement can publish a different
               current cut. The disposal event represents a reference, policy
               and producer fence recheck followed by successful owner I/O. */
        first start then candidate;
        state candidate;
        state current;
        state historical;
        state eligible;
        state disposed;
        transition qualify first candidate accept CutAdmitted then current;
        transition supersede first current accept CutSuperseded then historical;
        transition expire first historical accept DisposalEligible then eligible;
        transition retain first eligible accept KeepForReview then historical;
        transition dispose first eligible accept DisposalCommitted then disposed;
    }

    action def <'MAR-GCTX-F02'> CommitOwnerChange {
        action admitOwnerCut;
        action establishRequiredHold;
        action publishOwnerReceipt;
        action commitLocatorAndCursor;

        doc /* The owner receipt permits replay after a crash before index
               publication. Incomplete owner coverage fences disposal and
               prevents a current index claim for that scope. */
        first start then admitOwnerCut;
        first admitOwnerCut then establishRequiredHold;
        first establishRequiredHold then publishOwnerReceipt;
        first publishOwnerReceipt then commitLocatorAndCursor;
    }

    action def <'MAR-GCTX-F03'> QualifyOrgLink {
        action readSelectedOrgLink;
        action resolveWorktreeDeclaration {
            out resolved : Boolean;
            out contentOnly : Boolean;
            doc /* Lab follows a relative file link with a unique CUSTOM_ID,
                   admitted declaration, current selected source and grant.
                   Unsupported link forms can stay content. Invalid selected
                   source, selector or grant produces a refusal. */
        }
        action checkGitBinding {
            out qualified : Boolean;
            doc /* Real Repo proof is separate from the source key and
                   Worktree label recorded by a Lab fixture. */
        }
        action recordLinkContent;
        action recordAdmissionRefusal;
        action recordUnqualifiedRepoHop;
        action recordQualifiedHop;
        decide hasResolvedSource;
        decide unresolvedDisposition;
        decide hasRepoBinding;

        first start then readSelectedOrgLink;
        first readSelectedOrgLink then resolveWorktreeDeclaration;
        first resolveWorktreeDeclaration then hasResolvedSource;
        first hasResolvedSource if resolveWorktreeDeclaration.resolved then checkGitBinding;
        first hasResolvedSource if not resolveWorktreeDeclaration.resolved then unresolvedDisposition;
        first unresolvedDisposition if resolveWorktreeDeclaration.contentOnly then recordLinkContent;
        first unresolvedDisposition if not resolveWorktreeDeclaration.contentOnly then recordAdmissionRefusal;
        first checkGitBinding then hasRepoBinding;
        first hasRepoBinding if checkGitBinding.qualified then recordQualifiedHop;
        first hasRepoBinding if not checkGitBinding.qualified then recordUnqualifiedRepoHop;
    }

    action def <'MAR-GCTX-F04'> PublishCrossRepoOrigin {
        action observeBothRepositoryBindings;
        action checkBothDisplayGrants {
            out allowed : Boolean;
            doc /* Recheck the prepared scope against both current grants
                   at the effect boundary. A refusal performs zero reads. */
        }
        action openBothGitObjects;
        action receiveGitPayloads {
            doc /* Hold the returned bytes in the current attempt. Cache
                   and Lab source admission follow the delivery fence. */
        }
        action recheckReadDelivery {
            out current : Boolean;
            doc /* Reject both completions when either grant changed while
                   Git was reading. Retain the earlier admitted cache cut. */
        }
        action discardPayloads;
        action matchOriginAndContracts {
            out matched : Boolean;
            doc /* Match the explicit relation, the source and target owners,
                   qualified anchor, publication objects and Org digests. */
        }
        action admitSourceCuts;
        action evaluateSelectedOrgCuts;
        action recheckGrantAndCutCurrentness {
            out current : Boolean;
        }
        action recordRefusal;
        action publishLineageAndCache;
        decide originAccepted;
        decide grantsAdmitted;
        decide deliveryCurrent;
        decide stillCurrent;
        merge refused;
        first start then observeBothRepositoryBindings;
        first observeBothRepositoryBindings then checkBothDisplayGrants;
        first checkBothDisplayGrants then grantsAdmitted;
        first grantsAdmitted if checkBothDisplayGrants.allowed then openBothGitObjects;
        first grantsAdmitted if not checkBothDisplayGrants.allowed then refused;
        first openBothGitObjects then receiveGitPayloads;
        first receiveGitPayloads then recheckReadDelivery;
        first recheckReadDelivery then deliveryCurrent;
        first deliveryCurrent if recheckReadDelivery.current then matchOriginAndContracts;
        first deliveryCurrent if not recheckReadDelivery.current then discardPayloads;
        first discardPayloads then refused;
        first matchOriginAndContracts then originAccepted;
        first originAccepted if matchOriginAndContracts.matched then admitSourceCuts;
        first originAccepted if not matchOriginAndContracts.matched then refused;
        first admitSourceCuts then evaluateSelectedOrgCuts;
        first evaluateSelectedOrgCuts then recheckGrantAndCutCurrentness;
        first recheckGrantAndCutCurrentness then stillCurrent;
        first stillCurrent if recheckGrantAndCutCurrentness.current then publishLineageAndCache;
        first stillCurrent if not recheckGrantAndCutCurrentness.current then refused;
        first refused then recordRefusal;
    }

    action def <'MAR-GCTX-F01'> RestoreContext {
        in launchPath : String;
        in displayScope : String;
        action observeLaunchWorktree {
            out launch : GitObservation;
        }
        action findBoundedCandidates {
            doc /* Locate sources visible under the request scope, with
                   explicit coverage and a bounded candidate count. */
        }
        action bindAuthorizedSourceCuts {
            out ready : Boolean;
            doc /* Resolve each candidate against owner evidence. Check
                   display authorization before opening its protected bytes. */
        }
        action runLabQuery;
        action recheckCurrentRequest {
            out usable : Boolean;
            doc /* Check request, launch observation, source cuts, result
                   contract, coverage and current display authority. */
        }
        action publishContextView;
        action publishUnavailable;
        decide sourceReady;
        decide resultCurrent;
        merge rejected;

        first start then observeLaunchWorktree;
        first observeLaunchWorktree then findBoundedCandidates;
        first findBoundedCandidates then bindAuthorizedSourceCuts;
        first bindAuthorizedSourceCuts then sourceReady;
        first sourceReady if bindAuthorizedSourceCuts.ready then runLabQuery;
        first sourceReady if not bindAuthorizedSourceCuts.ready then rejected;
        first runLabQuery then recheckCurrentRequest;
        first recheckCurrentRequest then resultCurrent;
        first resultCurrent if recheckCurrentRequest.usable then publishContextView;
        first resultCurrent if not recheckCurrentRequest.usable then rejected;
        first rejected then publishUnavailable;
    }
}
