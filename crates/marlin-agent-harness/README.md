# Marlin Agent Harness
`marlin-agent-harness` is the agent AI harness crate. Its job is to drive agent-system scenarios through controlled runtime boundaries, capture typed evidence, and evaluate whether the observed agent behavior satisfies a scenario contract.
This is a different boundary from ordinary Rust tests, generic test support, and the project/package quality harness.
## Definition
An agent AI harness is a runtime-facing scenario driver. It owns the machinery needed to answer:
- What agent scenario was executed?
- Which runtime boundary did it use?
- Which provider, tool, hook, and sub-agent edges were visible?
- Which events, spans, evidence facts, release visibility facts, and receipts were captured?
- Did those observations satisfy the scenario contract?
The harness is therefore closer to an app-server test client, mock model server, request recorder, streaming SSE server, isolated home/config fixture, and process/API driver pattern than to a plain unit-test helper. That shape lets scenarios submit turns, isolate home/config/workspace state, mock the model boundary, capture requests and streams, and observe agent events without calling a live LLM.
Our version should keep that lesson, but make the boundaries more explicit: typed scenario contracts, typed evidence, typed visibility receipts, typed trace packages, and a clear split from generic test utilities.
## Owns
This crate owns agent-scenario execution and evaluation primitives:
- `AgentHarness`: evaluates scenario contracts against observed events, evidence, and spans.
- `HarnessRuntime`: creates a controlled runtime, runs graph-loop requests, records runtime environment visibility, drains runtime events, and returns typed execution reports.
- `HarnessExecutionReport`: packages graph-loop result, events, evidence, trace spans, summary data, and assertion diagnostics from one harness run.
- `TraceRecorder`: captures harness-local tracing spans without installing a process-global subscriber.
- `HarnessGraphBuilder`: builds compact graph fixtures used by agent scenarios.
- Static provider, tool, hook, and sub-agent runtimes when they model scenario actor boundaries inside the agent runtime contract.
- Release visibility evidence and release gate execution receipts when an agent scenario needs to prove what release topology and gate evidence it observed.
## Does Not Own
This crate must not become a generic testing crate.
- Reusable mock gateways, streaming gates, deterministic fake servers, fixture builders, and cross-crate assertions belong in `marlin-agent-test-support`.
- No-live-LLM gateway guards, replay fixtures, deterministic gateway denial evidence, and reusable graph-loop node receipt fixtures belong in `marlin-agent-test-support`. The harness consumes those receipts as typed runtime evidence; it must not replace them with ad hoc log parsing or live-provider calls.
- Package-wide three-layer testing coverage belongs in `marlin-agent-test-support`, with crates consuming those assertions from their own tests.
- Per-crate and per-package quality evidence graphs, performance gates, stability gates, build policies, and Nix/build integration belong in the Rust project harness policy layer, currently under `build-support/` and the `rust-lang-project-harness` integration.
- Production runtime implementations belong in the runtime, kernel, protocol, hooks, stream, and provider crates. The harness can exercise those boundaries, but it should not own their production behavior.
- Complex hook policy logic does not belong here. Rust-side hook configuration should stay typed and simple; complex policy belongs in the Gerbil Scheme extension plane.
## Placement Rules
Use these rules before adding new code:
- If the code drives an agent scenario and emits typed runtime evidence, it belongs in `marlin-agent-harness`.
- If the code is a reusable fake, gate, assertion, fixture, mock server, or test helper that multiple crates can use, it belongs in `marlin-agent-test-support`.
- If the code models no-live-LLM replay behavior, it should emit typed runtime evidence for denied gateway attempts and graph-loop node receipts before harness evaluation.
- If the code enforces package quality, crate topology, stability, performance, build reproducibility, or Nix-facing evidence, it belongs in the project harness policy layer, not here.
- If the code implements live runtime behavior, it belongs in the production runtime crate that owns that behavior; harness tests may exercise it.
- Harness tests may consume `marlin-agent-test-support`, but should not reimplement test-support primitives inside this crate.
## Evidence Boundary
The word `evidence` has two different meanings in this workspace:
- Agent evidence is what this crate captures from a scenario run: events, spans, runtime environment visibility, release visibility, and receipts that show what the agent observed or did.
- Package quality evidence is what the project harness policy captures for a crate or package: build checks, static policy facts, performance gates, stability gates, and reproducibility artifacts.
Both are important, but they are not the same subsystem. Agent evidence can be a signal inside tests. Package quality evidence is a build and engineering gate.
## Reference Responsibilities
Useful runtime harnesses share the same responsibility shape:
- build isolated home, config, environment, model, shell, and workspace state, then submit agent turns through the real runtime boundary;
- record model requests so tests can inspect prompt input, tools, instructions, and function calls;
- provide gated streaming chunks so tests can prove behavior across partial model output;
- drive real child processes or API servers with custom home, env, startup, and pending message tracking.
Those are harness responsibilities because they drive and observe the agent runtime boundary. They are not package quality gates and they are not the Rust test runner itself.
