# marlin-agent-core Agent Rules

## Protocol Evolution

- Do not keep compatibility shims when they preserve ambiguous behavior or duplicate protocol meaning.
- Prefer explicit migrations over mixed old/new APIs. Remove ambiguous constructors and replace them with typed request, config, or builder surfaces.
- Public semantic identifiers must use domain newtypes instead of primitive strings.
- Rust owns typed protocols, receipts, runtime boundaries, and `TOML` configuration envelopes.
- Do not invent a complex Rust-side hook DSL. Simple hook configuration belongs in typed `TOML`; complex policy logic belongs in the Gerbil Scheme extension plane.
- Dynamic hook policy actions must cross the Rust/Gerbil boundary as typed protocol receipts. `Register` and `Unregister` actions must resolve through a Rust-owned runtime catalog of existing `HookRegistration` handlers; never let Scheme, JSON, or compatibility shims manufacture Rust handlers.
- When a legacy hook/protocol surface becomes ambiguous, remove it in the same change that introduces the typed replacement. Do not keep old and new action meanings active together for compatibility.
