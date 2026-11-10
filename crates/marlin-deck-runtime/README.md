# Marlin Deck Runtime

This crate owns Rust model-route admission, the linked selector and Deck
Runtime metadata projection. Scheme publishes the Deck Runtime Module and
owns its version, actor pool and policy evaluation.

`DeckRouteAdmission` uses a bounded Tokio queue and a fixed thread for the
linked Scheme VM. The thread submits typed batches through the SDK's rooted
object ABI to the resident Scheme actors. It projects the resulting Boolean
vector and qualifies the candidate against the complete Rust route matcher
before returning a typed receipt. An expired or cancelled request skips VM
entry at dequeue. A batch already inside the VM retires on its owner.

With `deck-actor`, `marlin-agent-route` accepts a `DeckRouteAdmission` in
`ModelRouteHttpState::with_deck_admission` and serves it through the regular
chat route. Build the Scheme package with the paired Gerbil SDK first and
keep the compiled package and SDK modules on `GERBIL_LOADPATH`. Run the
linked HTTP test with `cargo test -p marlin-agent-route --features deck-actor
chat_admission_executes_the_linked_scheme_deck_actor`.

Run `cargo test -p marlin-deck-runtime` for the Rust contracts and
`cargo bench -p marlin-deck-runtime --features linked-selector --bench linked_selector`
for the linked policy selector benchmark.
