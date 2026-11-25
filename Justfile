# SPDX-FileCopyrightText: 2026 tao3k team and Contributors
#
# SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# Bound native Scheme builds to the available processors unless the caller
# has selected a smaller budget.
export GERBIL_BUILD_CORES := env_var_or_default("GERBIL_BUILD_CORES", num_cpus())
gerbil_prefix := env_var_or_default("GERBIL_PATH", justfile_directory() / "scheme" / ".gerbil")
export GERBIL_LOADPATH := env_var_or_default("GERBIL_LOADPATH", gerbil_prefix / "lib")

default:
  @just --list

# Validate the project entry and maintained OpenSysML models together.
sysml-check:
  sysml -strict -validate -self-check -no-record-cache project.sysml dev/sysml

# Run the ASP Rust native workspace Dev Gate through Cargo test.
rust-dev-gate:
  cargo test --locked -p marlin-rust-project-harness-policy --test workspace_policy

# Authored Quint models and reproducible trace checks.
property-model-gate: app-server-model-gate gitops-context-model-gate search-engine-model-gate ui-helm-model-gate graph-engine-model-gate
  quint typecheck dev/quint/host-observer.qnt
  quint run dev/quint/host-observer.qnt --seed=42 --max-samples=100 --max-steps=8 --invariant=safe --verbosity=1
  @output=$(quint run dev/quint/host-observer.qnt --step=badStep --invariant=safe --seed=42 --max-samples=100 --max-steps=8 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in host exit negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  quint typecheck dev/quint/runtime-process-owner.qnt
  quint test dev/quint/runtime-process-owner.qnt --main=runtimeProcessOwner
  quint run dev/quint/runtime-process-owner.qnt --main=runtimeProcessOwner --seed=42 --max-samples=1000 --max-steps=8 --invariant=safe --verbosity=1
  @output=$(quint run dev/quint/runtime-process-owner.qnt --main=runtimeProcessOwner --step=watchdogMutation --invariant=safe --seed=42 --max-samples=1000 --max-steps=12 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in watchdog negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  @output=$(quint run dev/quint/runtime-process-owner.qnt --main=runtimeProcessOwner --step=healthMutation --invariant=safe --seed=42 --max-samples=1000 --max-steps=12 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in health negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  @output=$(quint run dev/quint/runtime-process-owner.qnt --main=runtimeProcessOwner --step=badStep --invariant=safe --seed=42 --max-samples=1000 --max-steps=8 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in process owner negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  quint typecheck dev/quint/event-bus.qnt
  quint test dev/quint/event-bus.qnt --main=eventBus
  quint run dev/quint/event-bus.qnt --main=eventBus --seed=42 --max-samples=1000 --max-steps=10 --invariant=safe --verbosity=1
  @output=$(quint run dev/quint/event-bus.qnt --main=eventBus --step=badStep --invariant=safe --seed=42 --max-samples=1000 --max-steps=10 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in event bus negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  @test_binary=$(mktemp -t marlin-event-bus-test.XXXXXX); trap 'rm -f "$test_binary"' EXIT; rustc --edition=2024 --test dev/verification/event_bus_contract.rs -o "$test_binary"; "$test_binary"
  just tui-model-gate
  quint typecheck dev/quint/model-tool-review.qnt
  quint test dev/quint/model-tool-review.qnt --main=modelToolReview --max-samples=100
  quint run dev/quint/model-tool-review.qnt --main=modelToolReview --seed=42 --max-samples=1000 --max-steps=10 --invariant=safe --verbosity=1
  @output=$(quint run dev/quint/model-tool-review.qnt --main=modelToolReview --step=badStep --seed=42 --max-samples=1000 --max-steps=10 --invariant=safe --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in model tool review negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  quint typecheck dev/quint/strict-tree-proposal.qnt
  quint test dev/quint/strict-tree-proposal.qnt --main=strictTreeProposal
  quint run dev/quint/strict-tree-proposal.qnt --main=strictTreeProposal --seed=42 --max-samples=1000 --max-steps=12 --invariant=safe --verbosity=1
  @output=$(quint run dev/quint/strict-tree-proposal.qnt --main=strictTreeProposal --step=badLaunchStep --invariant=safe --seed=42 --max-samples=1000 --max-steps=12 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in strict launch negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  @output=$(quint run dev/quint/strict-tree-proposal.qnt --main=strictTreeProposal --step=badReleaseStep --invariant=safe --seed=42 --max-samples=1000 --max-steps=12 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in strict cleanup negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  quint typecheck dev/quint/process-lease.qnt
  quint test dev/quint/process-lease.qnt --main=processLease
  quint run dev/quint/process-lease.qnt --main=processLease --seed=42 --max-samples=1000 --max-steps=10 --invariant=safe --verbosity=1
  @output=$(quint run dev/quint/process-lease.qnt --main=processLease --step=badStep --invariant=safe --seed=42 --max-samples=1000 --max-steps=10 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in process lease negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  quint typecheck dev/quint/process-tree.qnt
  quint test dev/quint/process-tree.qnt --main=processTree
  quint run dev/quint/process-tree.qnt --main=processTree --seed=42 --max-samples=1000 --max-steps=8 --invariant=safe --verbosity=1
  @output=$(quint run dev/quint/process-tree.qnt --main=processTree --step=badStep --invariant=safe --seed=42 --max-samples=1000 --max-steps=8 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in process tree negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }
  quint typecheck dev/quint/process-quota.qnt
  quint test dev/quint/process-quota.qnt --main=processQuota --max-samples=100
  quint run dev/quint/process-quota.qnt --main=processQuota --invariant=safe --seed=42 --max-samples=1000 --max-steps=8 --verbosity=1
  @output=$(quint run dev/quint/process-quota.qnt --main=processQuota --step=badStep --invariant=safe --seed=42 --max-samples=1000 --max-steps=8 --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in process quota negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }

# MAR-UI-R10/R11/R25. Layout and echo share the same TUI property entry.
tui-model-gate:
  quint typecheck dev/quint/tui-minibuffer.qnt
  quint test dev/quint/tui-minibuffer.qnt --main=tuiMinibuffer --max-samples=100
  quint run dev/quint/tui-minibuffer.qnt --main=tuiMinibuffer --seed=42 --max-samples=1000 --max-steps=30 --invariant=stateSafe --verbosity=1
  quint test dev/quint/tui-minibuffer.qnt --main=minibufferEcho --max-samples=100
  quint run dev/quint/tui-minibuffer.qnt --main=minibufferEcho --seed=42 --max-samples=1000 --max-steps=30 --invariant=safe --verbosity=1
  @output=$(quint run dev/quint/tui-minibuffer.qnt --main=minibufferEcho --step=badStep --seed=42 --max-samples=100 --max-steps=4 --invariant=safe --verbosity=1 2>&1) && { printf '%s\n' "Unexpected success in echo timing negative control"; exit 1; } || { printf '%s\n' "$output"; printf '%s\n' "$output" | grep -q 'Invariant violated'; }

# Run only inside a disposable, root-owned delegated Linux systemd scope.
strict-linux-os-qualification:
  python3 dev/verification/linux_cgroup_qualification.py

# Session journal, reconnect cursor and Runtime cancellation.
app-server-model-gate:
  quint typecheck dev/quint/app-server-turn.qnt
  quint test dev/quint/app-server-turn.qnt --max-samples=100
  quint run dev/quint/app-server-turn.qnt --seed=42 --max-samples=1000 --max-steps=30 --invariant=safe --verbosity=1

# MAR-GCTX: versioned source admission, retirement and custody.
gitops-context-model-gate:
  quint typecheck dev/quint/gitops-context.qnt
  quint test dev/quint/gitops-context.qnt --main=gitopsContextAdmission
  quint test dev/quint/gitops-context.qnt --main=gitopsContextRetention
  quint test dev/quint/gitops-context.qnt --main=gitopsOrgLinkEvidence
  quint test dev/quint/gitops-context.qnt --main=gitopsCrossRepoPublication
  quint test dev/quint/gitops-context.qnt --main=gitopsWorkingCopyPolicy
  quint run dev/quint/gitops-context.qnt --main=gitopsContextAdmission --seed=42 --max-samples=1000 --max-steps=16 --invariant=safe --verbosity=1
  quint run dev/quint/gitops-context.qnt --main=gitopsContextRetention --seed=42 --max-samples=1000 --max-steps=16 --invariant=safe --verbosity=1
  quint run dev/quint/gitops-context.qnt --main=gitopsOrgLinkEvidence --seed=42 --max-samples=1000 --max-steps=16 --invariant=safe --verbosity=1
  quint run dev/quint/gitops-context.qnt --main=gitopsCrossRepoPublication --seed=42 --max-samples=1000 --max-steps=16 --invariant=safe --verbosity=1
  quint run dev/quint/gitops-context.qnt --main=gitopsWorkingCopyPolicy --seed=42 --max-samples=1000 --max-steps=16 --invariant=safe --verbosity=1
  @for control in gitopsContextAdmission:foreignCloneMutation gitopsContextAdmission:stalePublicationMutation gitopsContextAdmission:cursorMutation gitopsContextRetention:mergeMutation gitopsContextRetention:custodyMutation gitopsOrgLinkEvidence:attachmentMutation gitopsOrgLinkEvidence:crossRepoMutation gitopsCrossRepoPublication:latePublicationMutation gitopsCrossRepoPublication:preOpenMutation gitopsCrossRepoPublication:lateCacheMutation gitopsWorkingCopyPolicy:missingCoreMutation; do result=0; output=$(quint run dev/quint/gitops-context.qnt --main="${control%:*}" --step="${control#*:}" --seed=42 --max-samples=1000 --max-steps=16 --invariant=safe --verbosity=1 2>&1) || result=$?; printf '%s\n%s\n' "$control" "$output"; if [[ $result -ne 1 ]] || ! printf '%s\n' "$output" | grep -q 'Invariant violated'; then exit 1; fi; done

# The offline CLI checks Quint IR without starting an RPC server.
gitops-context-symbolic-gate apalache="apalache-mc":
  #!/usr/bin/env bash
  set -euo pipefail
  model_path="$PWD/dev/quint/gitops-context.qnt"
  proof_dir="$PWD/target/quint/gitops-context-verification"
  mkdir -p "$proof_dir"
  cd "$proof_dir"
  for module in gitopsContextAdmission gitopsContextRetention gitopsOrgLinkEvidence gitopsCrossRepoPublication gitopsWorkingCopyPolicy; do
    quint compile "$model_path" --main="$module" --invariant=safe --target=json > "$module.qnt.json"
    {{ quote(apalache) }} check --init=init --next=step --inv=safe --length=16 "$module.qnt.json" > "$module-apalache.log" 2>&1 || { tail -n 30 "$module-apalache.log"; exit 1; }
    grep -F "Checker reports no error up to computation length 16" "$module-apalache.log"
  done

# MAR-SRCH: converged native search engine CAS publication, Scheme ABI admission, and edge custody.
search-engine-model-gate:
  quint typecheck dev/quint/search-engine.qnt
  quint test dev/quint/search-engine.qnt --main=searchGenerationPublication
  quint test dev/quint/search-engine.qnt --main=searchPlanAbiAdmission
  quint test dev/quint/search-engine.qnt --main=searchTopologyEdgeCustody
  quint run dev/quint/search-engine.qnt --main=searchGenerationPublication --seed=42 --max-samples=1000 --max-steps=16 --invariant=safe --verbosity=1
  quint run dev/quint/search-engine.qnt --main=searchPlanAbiAdmission --seed=42 --max-samples=1000 --max-steps=16 --invariant=safe --verbosity=1
  quint run dev/quint/search-engine.qnt --main=searchTopologyEdgeCustody --seed=42 --max-samples=1000 --max-steps=16 --invariant=safe --verbosity=1
  @for control in searchGenerationPublication:publishWithoutBlobMutation searchGenerationPublication:bypassCasMutation searchGenerationPublication:nonMonotonicRevisionMutation searchPlanAbiAdmission:bypassAdmissionMutation searchPlanAbiAdmission:staleGenerationMutation searchTopologyEdgeCustody:crossGenerationLeakMutation searchTopologyEdgeCustody:duplicateCountMutation; do result=0; output=$(quint run dev/quint/search-engine.qnt --main="${control%:*}" --step="${control#*:}" --seed=42 --max-samples=1000 --max-steps=16 --invariant=safe --verbosity=1 2>&1) || result=$?; printf '%s\n%s\n' "$control" "$output"; if [[ $result -ne 1 ]] || ! printf '%s\n' "$output" | grep -q 'Invariant violated'; then exit 1; fi; done

# MAR-HELM: Modern Helm epoch fencing, closed-session immunity, and mark-pruning safety.
ui-helm-model-gate:
  quint typecheck dev/quint/ui-helm.qnt
  quint test dev/quint/ui-helm.qnt --main=uiHelmEpochSafety
  quint run dev/quint/ui-helm.qnt --main=uiHelmEpochSafety --seed=42 --max-samples=1000 --max-steps=16 --invariant=safe --verbosity=1
  @for control in uiHelmEpochSafety:staleResultOverwriteMutation uiHelmEpochSafety:applyWhileClosedMutation uiHelmEpochSafety:danglingMarkMutation; do result=0; output=$(quint run dev/quint/ui-helm.qnt --main="${control%:*}" --step="${control#*:}" --seed=42 --max-samples=1000 --max-steps=16 --invariant=safe --verbosity=1 2>&1) || result=$?; printf '%s\n%s\n' "$control" "$output"; if [[ $result -ne 1 ]] || ! printf '%s\n' "$output" | grep -q 'Invariant violated'; then exit 1; fi; done

# MAR-GRAPH: Session-free TaskActor fencing-epoch anti-split-brain, C4 steering convergence, governed context/detour confinement, and Workbench tab invariants.
graph-engine-model-gate:
  quint typecheck dev/quint/graph-engine.qnt
  quint test dev/quint/graph-engine.qnt --main=graphEngineTaskActorSafety
  quint test dev/quint/graph-engine.qnt --main=graphEngineWorkbenchTabSafety
  quint test dev/quint/graph-engine.qnt --main=graphEngineGovernedContextSafety
  quint run dev/quint/graph-engine.qnt --main=graphEngineTaskActorSafety --seed=42 --max-samples=1000 --max-steps=16 --invariant=safe --verbosity=1
  quint run dev/quint/graph-engine.qnt --main=graphEngineWorkbenchTabSafety --seed=42 --max-samples=1000 --max-steps=16 --invariant=safe --verbosity=1
  quint run dev/quint/graph-engine.qnt --main=graphEngineGovernedContextSafety --seed=42 --max-samples=1000 --max-steps=16 --invariant=safe --verbosity=1
  @for control in graphEngineTaskActorSafety:staleReceiptAcceptMutation graphEngineTaskActorSafety:prematureJoinAdvanceMutation graphEngineTaskActorSafety:detachedProjectCommitMutation graphEngineTaskActorSafety:postTerminalMutation graphEngineWorkbenchTabSafety:unpinnedIndexMutation graphEngineWorkbenchTabSafety:peekOverlapMutation graphEngineGovernedContextSafety:defeaterBelowL3Mutation graphEngineGovernedContextSafety:lethalTrifectaBypassMutation graphEngineGovernedContextSafety:settleDuringBlockerDetourMutation; do result=0; output=$(quint run dev/quint/graph-engine.qnt --main="${control%:*}" --step="${control#*:}" --seed=42 --max-samples=1000 --max-steps=16 --invariant=safe --verbosity=1 2>&1) || result=$?; printf '%s\n%s\n' "$control" "$output"; if [[ $result -ne 1 ]] || ! printf '%s\n' "$output" | grep -q 'Invariant violated'; then exit 1; fi; done

# Multi-scale Criterion benchmarks for Marlin converged search engine bridge.
search-engine-bench:
  cargo bench --locked -p marlin-agent-search-engine --bench search_engine_bridge

# One project gate: architecture, model, native Scheme PBT and Rust properties.
property-gate: sysml-check property-model-gate scheme-property-gate
  cargo test --locked -p marlin-agent-search-engine
  cargo test --locked -p marlin-agent-telemetry
  cargo test --locked -p marlin-agent-runtime --test unit_test process::quota
  cargo test --locked -p marlin-agent-runtime --test unit_test process::containment
  cargo test --locked -p marlin-agent-runtime --test unit_test process::managed_executor
  cargo test --locked -p marlin-agent-runtime --test unit_test runtime_task_tracker
  cargo test --locked -p marlin-agent-gitops
  cargo test --locked -p marlin-agent-runtime --lib confirmed_process_does_not_release_until_pipe_endpoint_is_dropped
  cargo test --locked -p marlin-agent-kernel --test unit_test strict_tree_
  just process-quota-gate
  cargo test --locked -p marlin-agent-tui --no-default-features --lib
  cargo test --locked -p marlin-agent-ui --test ui_native_projection
  if [[ $(uname -s) == Darwin ]]; then just tui-runtime-gate; fi

process-quota-gate:
  cargo test --locked -p marlin-agent-runtime --test process_quota_gate -- --ignored --nocapture

# Real linked macOS TUI: override MARLIN_TUI_SOAK_SECONDS for longer runs.
tui-runtime-gate:
  just scheme-build
  cargo test --locked -p marlin-agent-tui --test pty
  cargo test --locked -p marlin-agent-tui --test pty streaming_command_navigation_stays_within_admitted_candidates -- --ignored --nocapture
  MARLIN_TUI_SOAK_SECONDS="${MARLIN_TUI_SOAK_SECONDS:-30}" cargo test --locked -p marlin-agent-tui --test pty repeated_stream_cancellation_soak -- --ignored --nocapture
  cargo test --locked -p marlin-agent-tui --test pty workspace_tool_cancel_reaps_process -- --ignored --nocapture
  cargo test --locked -p marlin-agent-tui --test pty workspace_tool_shutdown_reaps_after_exit -- --ignored --nocapture
  cargo test --locked -p marlin-agent-tui --test pty stalled_collector_does_not_delay_runtime_drain_or_block_tui_exit -- --ignored --nocapture
  cargo test --locked -p marlin-agent-tui --test pty quiet_workspace_tool_survives_idle_observation -- --ignored --nocapture
  cargo test --locked -p marlin-agent-tui --test pty process_buffer_stops_a_managed_generation -- --ignored --nocapture
  @trace_dir=$(mktemp -d -t marlin-host-trace.XXXXXX); trap 'rm -rf "$trace_dir"' EXIT; quint test dev/quint/host-observer.qnt --out-itf="$trace_dir/{test}-{seq}.json"; trace_files=("$trace_dir"/*.json); [[ ${#trace_files[@]} == 1 && -f "${trace_files[0]}" ]]; MARLIN_HOST_OBSERVER_TRACE="${trace_files[0]}" cargo test --locked -p marlin-agent-tui --test pty independent_observer_distinguishes_stall_from_exit -- --ignored --nocapture

# Gerbil's native test runner discovers the selected package's maintained suite.
scheme-property-gate:
  just scheme-test
  just scheme-test t/ui-native-test.ss

# Gerbil owns discovery and assertions; invoke :gerbil/tools/gxtest directly so
# its structured return value (0 vs 42) governs the process exit status instead
# of `gerbil test` discarding it and scraping stdout with rg/grep.
scheme-test module="t/all-test.ss":
  #!/usr/bin/env bash
  set -euo pipefail
  watchdog=$(command -v timeout || command -v gtimeout)
  export GAMBOPT="${GAMBOPT:+$GAMBOPT,}max-heap=2G,debug=q"
  export MARLIN_SCHEME_TEST_MODULE={{ quote(module) }}
  cd scheme
  "$watchdog" --foreground --signal=TERM --kill-after=5s 600s gxpkg env gxi -:max-heap=2G,debug=q -e '(import :gerbil/tools/gxtest) (let (rc (main (getenv "MARLIN_SCHEME_TEST_MODULE"))) (exit (if (eqv? rc 0) 0 (if (fixnum? rc) rc 1))))'

# Install the dependency declared by the workspace Scheme package.
scheme-deps:
  #!/usr/bin/env bash
  set -euo pipefail
  if [[ "$(uname -s)" == Darwin ]]; then
    unset SDKROOT DEVELOPER_DIR NIX_CFLAGS_COMPILE
    export PATH="${GERBIL_PREFIX:+$GERBIL_PREFIX/bin:}/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"
  fi
  cd scheme
  gerbil deps --install

# Run the package's build and test through the embedded Gerbil VM.
scheme-ui-package-gate:
  cd scheme && gxpkg env cargo test --locked -p marlin-agent-tui --no-default-features --lib ui_host::tests::real_vm_routes_a_contributed_method_without_submitting_the_draft -- --ignored --exact --nocapture

# The fixture owns platform linking; the ordinary Gerbil runner owns tests.
scheme-ui-abi-gate:
  #!/usr/bin/env bash
  set -euo pipefail
  original_path="$PATH"
  if [[ "$(uname -s)" == Darwin ]]; then
    unset SDKROOT DEVELOPER_DIR NIX_CFLAGS_COMPILE
    export PATH="${GERBIL_PREFIX:+$GERBIL_PREFIX/bin:}/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"
  fi
  (cd scheme && gxpkg env gxi t/build.ss)
  export PATH="$original_path"
  just scheme-test t/ui-c-abi-test.ss

scheme-package-gate:
  just scheme-test

# Build the init-selected Scheme graph through the package Builder.
scheme-build: (_scheme-package "build")

# Remove the init-selected Scheme build and link products.
scheme-clean: (_scheme-package "clean")

[private]
_scheme-package action:
  #!/usr/bin/env bash
  set -euo pipefail
  if [[ "$(uname -s)" == Darwin ]]; then
    unset SDKROOT DEVELOPER_DIR NIX_CFLAGS_COMPILE
    export PATH="${GERBIL_PREFIX:+$GERBIL_PREFIX/bin:}/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"
  fi
  cd scheme
  gerbil {{ quote(action) }}

# Run the fixed full Context workload after provisioning the paired Scheme package.
context-latency-gate:
  #!/usr/bin/env bash
  set -euo pipefail
  watchdog=$(command -v timeout || command -v gtimeout)
  mkdir -p target
  evidence=$(mktemp -d target/context-cohort.XXXXXX)
  printf 'Context cohort evidence: %s\n' "$evidence"
  git rev-parse HEAD > "$evidence/revision"
  shasum -a 256 Cargo.lock > "$evidence/cargo-lock.sha256"
  cargo bench --locked -p marlin-deck-runtime --features actor-runtime \
    --bench single_vm_actor --no-run --message-format=json > "$evidence/build.jsonl"
  binary=$(jq -ser '[.[] | select(.reason == "compiler-artifact" and
    .target.name == "single_vm_actor" and .target.kind == ["bench"] and .executable != null)
    | .executable] | if length == 1 then .[0] else error("expected one Context benchmark") end' \
    "$evidence/build.jsonl")
  shasum -a 256 "$binary" > "$evidence/benchmark.sha256"
  export GAMBOPT="${GAMBOPT:+$GAMBOPT,}max-heap=2G,debug=q"
  printf '%s\n' "$GAMBOPT" > "$evidence/gambopt"
  failed=0
  for process in 1 2 3 4; do
    log="$evidence/process-$process.log"
    status=0
    "$watchdog" 180 "$binary" 64 4096 > "$log" 2>&1 || status=$?
    printf '%s\t%s\n' "$process" "$status" >> "$evidence/exit-status.tsv"
    if (( status == 0 )); then
      if ! awk '/^batch_capacity=64 requests=4096 / {
        for (i = 1; i <= NF; i++) {
          if ($i ~ /^p99_us=[0-9]+(\.[0-9]+)?$/) {
            split($i, field, "="); p99 = field[2]; samples++
          }
        }
      } END { exit !(samples == 1 && p99 < 20000) }' "$log"; then
        failed=1
      fi
    else
      printf 'Context process %s exited with status %s\n' "$process" "$status" >&2
      failed=1
    fi
    cat "$log"
  done
  exit "$failed"

# Build the default linked debug binary used by target/debug/marlin-tui.
tui-aot-debug: scheme-build
  cargo build --locked -p marlin-agent-tui --bin marlin-tui

# Link the staged Scheme graph and VM into the terminal application.
tui-aot-build: scheme-build
  cargo build --locked -p marlin-agent-tui --bin marlin-tui --features linked-ui --release

# Stage the linked Release with filesystem compression where Darwin supports it.
tui-stage-release directory binary="target/release/marlin-tui":
  #!/usr/bin/env bash
  set -euo pipefail
  binary={{ quote(binary) }}
  directory={{ quote(directory) }}
  if [[ ! -f "$binary" || ! -x "$binary" ]]; then
    printf 'Expected a built executable at %s\n' "$binary" >&2
    exit 1
  fi
  mkdir -p -- "$directory"
  destination="$directory/marlin-tui"
  if [[ -d "$destination" ]]; then
    printf 'Expected an executable path at %s\n' "$destination" >&2
    exit 1
  fi
  staging=$(mktemp -d "$directory/.marlin-tui.XXXXXX")
  trap 'rm -f -- "$staging/marlin-tui"; rmdir -- "$staging"' EXIT
  case "$(uname -s)" in
    Darwin) /usr/bin/ditto --hfsCompression "$binary" "$staging/marlin-tui" ;;
    *) install -m 0755 -- "$binary" "$staging/marlin-tui" ;;
  esac
  mv -f -- "$staging/marlin-tui" "$destination"

# Build the same linked binary as distribution while measuring Rust UI mapping.
tui-ui-mapping-bench: scheme-build
  cargo bench --locked -p marlin-agent-tui --bench ui_mapping

# Keep interaction latency, first-copy startup, and source-free release
# qualification in one reproducible command. Build time is not a sample.
tui-performance-bench: tui-ui-mapping-bench
  #!/usr/bin/env bash
  set -euo pipefail
  artifacts=$(mktemp)
  trap 'rm -f "$artifacts"' EXIT
  # Cargo's test build can replace target/debug/marlin-tui with a different artifact.
  cargo test --locked -p marlin-agent-tui --test launch --test pty --no-run --message-format=json > "$artifacts"
  test_binary() {
    jq -er --arg name "$1" 'select(.reason == "compiler-artifact" and .target.name == $name and .executable != null) | .executable' "$artifacts"
  }
  launch=$(test_binary launch)
  pty=$(test_binary pty)
  cargo build --locked -p marlin-agent-tui --bin marlin-tui
  cargo build --locked -p marlin-agent-tui --bin marlin-tui --features linked-ui --release
  "$launch" linked_debug_pre_ui_launch_pair --ignored --nocapture
  "$launch" linked_release_pre_ui_launch_pair --ignored --nocapture
  "$pty" first_mx_arrow_is_not_lost_before_the_menu_frame
  "$pty" mx_navigation_pty_latency --ignored --nocapture
  "$pty" debug_linked_fresh_copy_startup_pair --ignored --nocapture
  "$pty" release_linked_fresh_copy_startup_pair --ignored --nocapture
  "$pty" release_linked_ui_first_frame_without_a_gerbil_package_tree --ignored --nocapture
  "$pty" idle_output_probe --ignored --nocapture
  cargo bench --locked -p marlin-agent-runtime --bench runtime_events

# Diagnose whether the size-oriented profile improves a source-matched linked
# TUI. Stage the Scheme programme first; comparing against a stale native
# manifest can produce a misleading projection failure.
tui-size-performance-bench: scheme-build
  cargo build --locked -p marlin-agent-tui --bin marlin-tui --features linked-ui --profile release-size
  cargo test --locked -p marlin-agent-tui --test launch linked_release_size_pre_ui_launch_pair -- --ignored --nocapture
  cargo test --locked -p marlin-agent-tui --test pty release_size_linked_fresh_copy_startup_pair -- --ignored --nocapture

# Exercise the linked AgentGraph policy selector through its typed C ABI.
agent-policy-routing-abi-gate:
  direnv exec . cargo test --locked -p marlin-agent-policy-routing-abi --features linked-selector

# Benchmark build gate for AgentGraph policy-routing linked native selector.
agent-policy-routing-abi-bench-build:
  direnv exec . cargo bench -p marlin-agent-policy-routing-abi --features linked-selector --bench linked_selector --no-run

# Allocation-shape performance gate for AgentGraph policy-routing linked native selector.
agent-policy-routing-abi-performance-gate:
  direnv exec . cargo bench --locked -p marlin-agent-policy-routing-abi --features linked-selector --bench allocation_shape

# Full benchmark gate for AgentGraph policy-routing linked native selector.
agent-policy-routing-abi-bench:
  direnv exec . cargo bench -p marlin-agent-policy-routing-abi --features linked-selector --bench linked_selector

# Local P0-P4 gate for AgentGraph native policy-routing and Gerbil module-system closure.
agent-graph-abi-routing-gate: scheme-package-gate agent-policy-routing-abi-gate agent-policy-routing-abi-bench-build agent-policy-routing-abi-performance-gate
