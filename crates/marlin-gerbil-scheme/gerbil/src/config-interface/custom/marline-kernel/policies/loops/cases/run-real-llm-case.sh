#!/bin/sh
set -eu

profile_id="${1:?profile id required}"
case_id="${2:?case id required}"
max_rounds="${MARLIN_REAL_LLM_MAX_ROUNDS:-3}"
success_exit_status=0

case "$case_id" in
  marlin-runtime-handoff-real-llm)
    task_goal="recover a failed runtime handoff by producing a typed receipt repair plan"
    failure_fixture="runtime handoff failed because the manifest omitted catalog_resolution_receipt"
    acceptance="observe the failure, add catalog_resolution_receipt, and verify typed receipt ownership"
    ;;
  marlin-policy-receipt-gate-real-llm)
    task_goal="recover a policy receipt validation failure by identifying missing typed evidence"
    failure_fixture="policy projection failed because policy_projection_receipt was present but budget_receipt was missing"
    acceptance="identify the missing budget_receipt and return a receipt gate repair"
    ;;
  marlin-loop-contract-real-llm)
    task_goal="recover a loop contract publication failure by producing the missing contract summary"
    failure_fixture="loop contract publication failed because replay and receipt ownership were not summarized"
    acceptance="produce the missing contract summary and verify the publication boundary"
    ;;
  marlin-failure-retry-real-llm)
    task_goal="recover a failed loop iteration by observing the failure and retrying under typed budget"
    failure_fixture="loop iteration failed after the live LLM observation because the runtime tool returned a retryable non-zero exit status"
    acceptance="observe the failed iteration, propose retry, and preserve typed failure evidence"
    success_exit_status=17
    ;;
  *)
    echo "marlin-real-llm-case.result=fail"
    echo "marlin-real-llm-case.error=unknown-case:$case_id"
    exit 2
    ;;
esac

round=1
previous_output=""

while [ "$round" -le "$max_rounds" ]; do
  prompt=$(cat <<EOF
You are the live LLM worker inside a Marlin graph-loop policy experiment.
Do not call tools. Use only the information in this prompt.

Profile: $profile_id
Case: $case_id
Goal: $task_goal
Simulated failing loop state: $failure_fixture
Acceptance rule: $acceptance
Round: $round of $max_rounds
$previous_output

Return concise evidence lines. The output must include these exact keys:
marlin-real-llm-case.case_id=$case_id
marlin-real-llm-case.profile=$profile_id
marlin-real-llm-case.failure_observed=yes
marlin-real-llm-case.repair_proposed=yes
marlin-real-llm-case.verification=pass
marlin-real-llm-case.policy_observation=<one short sentence>
EOF
)

  output=$(codex exec --ephemeral --ignore-rules --sandbox read-only -- "$prompt" || true)

  if printf '%s\n' "$output" | grep -F "marlin-real-llm-case.case_id=$case_id" >/dev/null \
    && printf '%s\n' "$output" | grep -F "marlin-real-llm-case.failure_observed=yes" >/dev/null \
    && printf '%s\n' "$output" | grep -F "marlin-real-llm-case.repair_proposed=yes" >/dev/null \
    && printf '%s\n' "$output" | grep -F "marlin-real-llm-case.verification=pass" >/dev/null; then
    printf '%s\n' "$output"
    echo "marlin-real-llm-case.result=pass"
    echo "marlin-real-llm-case.rounds_used=$round"
    exit "$success_exit_status"
  fi

  previous_output=$(cat <<EOF
Previous round failed the acceptance marker check. Previous output:
$output
EOF
)
  round=$((round + 1))
done

printf '%s\n' "$output"
echo "marlin-real-llm-case.result=fail"
echo "marlin-real-llm-case.rounds_used=$max_rounds"
exit 1
