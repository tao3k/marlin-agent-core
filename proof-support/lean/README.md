# Marlin Lean Proof Support

This package is a proof-support boundary for Marlin reliability gates. It is not
part of any Rust crate `src` tree and does not define agent runtime behavior.

The first proof slice models artifact pointer compare-and-swap semantics used by
agent storage:

- `committed`: the pointer moves to the requested target only when the current
  value matches the expected value.
- `conflict`: the pointer stays unchanged when the expected value does not
  match the current value.
- `missing_expected`: a create-only operation succeeds only when the pointer is
  absent.

Run:

```sh
lake build
```

The intended integration path is:

1. keep Lean proofs in this isolated support package;
2. emit proof receipts into the Rust evidence graph only after `lake build`
   succeeds;
3. connect Rust storage scenario fixtures to Lean proof names through typed
   receipts, not ad hoc log text.
