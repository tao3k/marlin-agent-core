# Org-first Project Memory Runtime Design

Date: 2026-06-15
Status: draft for implementation alignment

## Design Position

Org is the core reference substrate for Marlin runtime work. It is not just a
fixture format, memory shard container, or debug input path. Org is the
parser-owned fact plane that records structured work metadata, evidence,
decisions, trace records, memory candidates, source spans, contract references,
and validation receipts.

The project memory runtime should therefore be designed from the Org substrate
outward:

1. Org documents define durable facts and references.
2. Org contracts define what agents must record so later recall is precise.
3. Org store discovers the project fact roots and shard families.
4. Org memory builds a graph read model from typed Org nodes.
5. Runtime graph query consumes intent and context, not file lists.
6. The agent receives a compact, source-spanned, contract-validated context
   pack, not raw transcripts or whole memory shards.

This design replaces the local "minimal slice" mindset with an architecture
rule: code slices are valid only when they move one of these substrate layers
toward the full Org-first runtime.

## Existing Anchors

- `docs/20-workspace/20.10-org-native-agent-workspace.org` says Org is the
  structured work metadata plane and records evidence, decisions, traces,
  memory candidates, verification, and source spans.
- `docs/40-rfcs/40.80-marlin-architecture-principles.org` says Marlin turns
  parser-owned facts, typed contracts, configurable policy, and supervised
  execution into auditable agent work.
- `docs/40-rfcs/40.120-project-scoped-agent-graph-runtime.org` says the
  differentiator is a small, source-spanned, contract-validated fact view,
  not more context.
- `docs/20-workspace/20.60-org-contracts-and-templates.org` gives contracts a
  parser-owned projection, validation report, matched node ids, templates,
  diagnostics, and status surface.
- `org/contracts/agent.memory.v1.org` defines memory candidates as recallable
  records with source, evidence, trust, retention, salience, and recall query
  fields.

## Architecture

### Layer 1: Org Fact Substrate

Org documents are the durable, inspectable reference layer. They should hold
the facts that make later runtime decisions auditable:

- work metadata: goals, plans, tasks, checklists, decisions, and SDD;
- evidence: source spans, command receipts, test receipts, trace records, and
  links to durable artifacts;
- memory candidates: claims, recall query terms, salience, trust, retention,
  and source references;
- contracts: validation rules, expected fields, matched nodes, unresolved
  references, template hints, and diagnostics.

The runtime should not treat Org as only text. The parser projection is the API
boundary: stable Rust domain types are built from Org nodes, properties, links,
backlinks, source spans, contract registries, and validation receipts.

### Layer 2: Contract-indexed Frontier

Contracts are how Marlin makes fast search precise. An agent that records
important work must also record the facts later recall needs:

- `RECALL_QUERY` terms for cheap candidate discovery;
- stable IDs for project, workspace, worktree, root session, session, content,
  memory, evidence, and artifacts;
- evidence links and source spans for inspection without reopening raw
  transcripts;
- salience, trust, retention, and validation state;
- backlinks and relationship hints when one fact explains or supersedes
  another.

`fd`, `rg`, and Org element query remain important, but they are frontier
builders. They should search the contracted fields and known project roots,
then hand candidates to the graph layer. They are not the final memory
semantics.

### Layer 3: Store-owned Discovery

Production recall must not ask callers to pass repeated `--org-memory` shard
paths. Explicit file inputs are useful fixtures, but the runtime API must be:

```text
ProjectMemoryRecallRequest
  GraphQueryContext
  recall_intent
  query_terms
  visible_surfaces
  fallback_policy
  result_budget
```

`marlin-org-store` owns discovery of eligible Org roots:

- project memory roots;
- session summary roots;
- worktree provenance roots;
- capability cards;
- contract registries;
- evidence and receipt roots.

The store returns typed document candidates and provenance. `marlin-org-memory`
loads projected Org nodes from that set and builds the read model.

### Layer 4: Graph Rerank And Visibility

The graph layer ranks candidate facts by typed relationships, not by raw text
position:

- same project;
- same workspace;
- same worktree provenance;
- same root session;
- child or parent session lineage;
- same content ancestry;
- explicit backlink;
- contract validated;
- high salience;
- trust and retention fit;
- external project import policy.

Sibling root sessions remain isolated at the raw transcript boundary. They may
share promoted project memory, source-spanned evidence, and compact context
packs only through explicit policy and contract-validated facts.

### Layer 5: Context Pack Output

The output of recall is a compact context pack:

- short claim or summary;
- source span and source reference;
- evidence ids and artifact links;
- relationship facts and score basis points;
- contract validation state;
- trust and retention hints;
- receipt expectations for follow-up tools.

It must not return whole Org documents, whole transcripts, or grep-style dumps.

## Crate Ownership

- `marlin-org-model`: stable Org projection DTOs and typed source spans.
- `marlin-org-workspace`: parser adapter, contract projection, reference
  resolution, validation receipts, and document loading.
- `marlin-org-store`: discovery of project Org roots and shard families.
- `marlin-org-memory`: graph read model over typed Org nodes; frontier, rerank,
  visibility, and context packing for memory facts.
- `marlin-agent-protocol`: public request, response, id, receipt, score, and
  relationship types.
- `marlin-agent-runtime`: session/content/runtime integration and budget-aware
  query execution.
- `marlin-agent-core`: CLI/debug facade only. It should not own recall
  semantics or shard discovery.

## Implementation Direction

The next code work should be architecture-bearing, not only local:

1. Define `ProjectMemoryRecallRequest` and `ProjectMemoryContextPack` protocol
   types around intent, context, budget, and compact output.
2. Move explicit multi-file loading behind a debug fixture name. Do not let it
   look like the production API.
3. Add store-backed Org root discovery for project memory, contracts, session
   summaries, and evidence roots.
4. Build a contract-indexed frontier in `marlin-org-memory` that uses
   `RECALL_QUERY`, tags, properties, links, backlinks, evidence ids, and source
   spans before graph rerank.
5. Pack recall output into typed context packs and source-spanned receipts.
6. Add tests that prove cross-session project memory works without leaking raw
   sibling transcripts.

## Testing Strategy

Tests should encode architectural boundaries:

- recall by `RECALL_QUERY` works when the claim text does not contain the query;
- unrelated property values do not accidentally match recall;
- same project across worktrees ranks strongly;
- sibling root raw transcript stays hidden;
- promoted memory from a sibling root session is visible through validated
  project memory facts;
- store discovery replaces caller-provided shard lists in production paths;
- debug fixture file inputs remain available but are named as fixtures;
- context packs contain claims, source spans, evidence ids, relationship
  reasons, and scores, but not whole documents.

## Non-goals

- Do not build sandbox backend selection in this track.
- Do not introduce a Rust-side hook DSL.
- Do not make `marlin-agent-core` own recall semantics.
- Do not treat `fd` or `rg` as final recall behavior.
- Do not return raw transcript or whole shard content as memory recall.

## Open Question

The first implementation decision is whether to land the protocol request and
context-pack types before store discovery, or to land store discovery first and
shape the protocol from the discovered root model. The recommended path is
protocol first: it forces the public API to stay intent/context based while the
store-backed discovery can evolve behind it.
