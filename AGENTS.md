<!--
SPDX-FileCopyrightText: 2026 tao3k team and Contributors

SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
-->

# marlin-agent-core Agent Rules

## Scheme Coding Style

See [Marlin Functional Scheme Style](scheme/CODING-STYLE.org) for the authoring conventions and examples.

- Use functional composition as the primary abstraction across Scheme modules. Express transformations, projections and state transitions as readable functions with explicit inputs and results.
- Keep module functions in `funcs.ss` and POO declarations in `objects.ss`. Let the interface publish the operations a consumer needs.
- Use POO declarations to group related values, contracts and callable slots. Let consumers compose those operations through module interfaces. Use slot-backed generics for extensible receiver behavior and CLOS methods when several arguments determine the behavior.
- Keep state changes and I/O at their owning boundaries. Compose pure operations over values and return the next state when the caller owns the transition.
- Reuse Gerbil `:std` and SRFI procedures for standard operations. Prefer a direct, named function when it explains the behavior clearly.
- Keep application behavior and presentation policy in Scheme where they fit the existing host capabilities. Use the Rust boundary for typed admission, native effects and reusable mappings.
- Keep `scheme/build.ss` minimal on Yuanxu's `defbuild-script` + `std-make-spec` + `.with-link` boundary (`compile` / `clean` / `spec` are owned by Yuanxu). Never add `required-lab-sentinels`, ad hoc `ensure-lab-dependencies!` shellouts, or manual upstream `.ssi` build/cleanup lists inside `scheme/build.ss`; rebuild/clean linked packages through `gerbil pkg build` and `just scheme-clean` / `just scheme-build`.
- Keep Scheme TUI behavior tests in the Gerbil suite under `scheme/t`. Add keymap, command, buffer, and layout regressions there, using Scheme fixtures with the shared test window for rendering checks. A Scheme-only UI change needs only Scheme test edits. Rust TUI tests cover the native boundary, terminal I/O, and generic Ratatui mapping.

## Protocol Evolution

- Do not keep compatibility shims when they preserve ambiguous behavior or duplicate protocol meaning.
- Prefer explicit migrations over mixed old/new APIs. Remove ambiguous constructors and replace them with typed request, config, or builder surfaces.
- Public semantic identifiers must use domain newtypes instead of primitive strings.
- Rust owns typed protocols, receipts, runtime boundaries, and `TOML` configuration envelopes.
- Gerbil Scheme must not own text serialization for runtime or native-internal APIs. Core Gerbil integration crosses as Scheme types -> Rust types and native ABI projections; Rust owns any outer configuration, trace, CLI, or explicit cross-process encoding.
- Native Gerbil graph-loop proposals must cross a Rust-owned native ABI readiness gate before an execution request is produced. Upper facade APIs should expose the readiness-aware compilation path instead of the raw proposal compiler.
- Gerbil replay artifacts should preserve typed runtime evidence, including no-live-LLM gateway denial receipts and per-node graph execution receipts. Do not replace those receipts with unstructured log text.
- Do not invent a complex Rust-side hook DSL. Simple hook configuration belongs in typed `TOML`; complex policy logic belongs in the Gerbil Scheme extension plane.
- Dynamic hook policy actions must cross the Rust/Gerbil boundary as typed protocol receipts. `Register` and `Unregister` actions must resolve through a Rust-owned runtime catalog of existing `HookRegistration` handlers; never let Scheme, ad hoc text encodings, or compatibility shims manufacture Rust handlers.
- When a legacy hook/protocol surface becomes ambiguous, remove it in the same change that introduces the typed replacement. Do not keep old and new action meanings active together for compatibility.
