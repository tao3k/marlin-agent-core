use marlin_org_model::OrgContractValidationStatus;
use marlin_org_workspace::{
    OrgDocument, OrgDocumentLoader, OrgDocumentWorkspace, STANDARD_AGENT_LOOP_CONTRACT_DOCUMENT_ID,
    STANDARD_AGENT_MEMORY_CONTRACT_DOCUMENT_ID, STANDARD_AGENT_PLAN_CONTRACT_DOCUMENT_ID,
    STANDARD_AGENT_TASK_CONTRACT_DOCUMENT_ID, STANDARD_AGENT_TOPOLOGY_CONTRACT_DOCUMENT_ID,
    load_standard_agent_contract_workspace, standard_agent_contract_documents,
};
use std::collections::BTreeSet;

#[test]
fn org_document_loader_loads_standard_agent_contract_library_from_org_folder() {
    let workspace =
        load_standard_agent_contract_workspace().expect("standard agent contract library loads");

    assert_eq!(
        contract_ids(&workspace),
        BTreeSet::from([
            "agent.loop.v1".to_owned(),
            "agent.memory.v1".to_owned(),
            "agent.plan.v1".to_owned(),
            "agent.task.v1".to_owned(),
            "agent.topology.v1".to_owned(),
        ])
    );
    assert!(workspace.contract_resolutions.references.is_empty());
    assert!(workspace.contract_validations.receipts.is_empty());
    assert_eq!(assertion_ids(&workspace).len(), 64);
    assert!(assertion_ids(&workspace).contains("plan.has-workflow-state"));
    assert!(assertion_ids(&workspace).contains("plan.has-next-action-property"));
    assert!(assertion_ids(&workspace).contains("plan.validation-has-checked-item"));
    assert!(assertion_ids(&workspace).contains("task.acceptance-has-checklist"));
    assert!(assertion_ids(&workspace).contains("loop.has-strategy"));
    assert!(assertion_ids(&workspace).contains("memory.has-retention"));
    assert!(assertion_ids(&workspace).contains("topology.has-source-anchor-property"));
    assert_eq!(
        standard_document_ids(),
        BTreeSet::from([
            STANDARD_AGENT_LOOP_CONTRACT_DOCUMENT_ID.to_owned(),
            STANDARD_AGENT_MEMORY_CONTRACT_DOCUMENT_ID.to_owned(),
            STANDARD_AGENT_PLAN_CONTRACT_DOCUMENT_ID.to_owned(),
            STANDARD_AGENT_TASK_CONTRACT_DOCUMENT_ID.to_owned(),
            STANDARD_AGENT_TOPOLOGY_CONTRACT_DOCUMENT_ID.to_owned(),
        ])
    );
}

#[test]
fn org_document_loader_validates_standard_agent_plan_task_loop_memory_topology_contracts() {
    let contract_documents = standard_agent_contract_documents();
    let work_document = OrgDocument::new(
        "doc:agent-contract-workspace",
        r#"* TODO Agent contract plan [0/5] [0%] :agent:org:contract:
:PROPERTIES:
:CONTRACT_ORG: agent.plan.v1
:ID: agent-contract-plan-20260614
:OBJECTIVE: validate the standard agent contract workspace fixture
:SDD: org/contracts/agent.plan.v1.org
:STABLE_REF: crates/marlin-org-workspace/tests/unit/document/contracts/standard_library.rs
:SCOPE_REF: marlin-org-workspace standard contract loader fixture
:PACKAGE: marlin-org-workspace
:SLICE: standard-agent-contract-plan-ledger
:COOKIE_DATA: direct
:STATUS: active
:NEXT_ACTION: continue: Contract Org adoption from standard library validation receipts.
:RECOVERY_REF: marlin-org-workspace-standard-agent-contract-plan-ledger
:RESUME_QUERY: marlin-org-workspace standard agent contract plan ledger
:EVIDENCE: standard_library.rs validates the agent plan ledger contract.
:EVIDENCE_STATUS: pending
:REVIEW_STATUS: pending
:END:

- [X] Scope and recovery anchor confirmed.
- [X] Task-local research complete.
- [ ] Implementation complete.
- [ ] Validation complete.
- [ ] Evidence and archive state updated.

** Context
Standard Contract Org needs to validate the same ledger shape agents use for real work, including recovery, evidence, and native Org progress surfaces.

** Checkpoints
- 2026-06-14: Real agent org samples use TODO/DONE roots, progress cookies, lifecycle checklist items, and recovery properties.

** Validation
- [X] Standard contract documents are discovered from independent org/contracts files.
- [X] Plan, task, loop, and memory contracts all pass in this fixture.

** Evidence
- standard_library.rs validates contract receipts without a live LLM.
- orgize owns contract query evaluation.

** Evidence Loop
:PROPERTIES:
:EVIDENCE_LOOP_ID: marlin-org-workspace-standard-agent-contract-plan-ledger-loop
:EVAL_STATUS: pending
:END:
| Claim | Evidence | Command | Result |
| The standard contract workspace fixture validates the plan ledger shape. | standard_library.rs fixture | cargo test -p marlin-org-workspace org_document_loader_validates_standard_agent_plan_task_loop_memory_topology_contracts | pending |

** Reflection
:PROPERTIES:
:REVIEW_MODE: active-plan-reflection
:REVIEW_STATUS: pending
:END:
| Question | Value | Evidence |
| Did the task finish? | no | active fixture state |
| Did project scope drift? | no | SCOPE_REF property |
| Are all checklist items done? | no | root checklist |
| Is evidence sufficient and replayable? | pending | Evidence Loop table |
| Is the plan archive-ready? | no | STATUS active |

** Recovery
#+begin_src text
recovery-ref: marlin-org-workspace-standard-agent-contract-plan-ledger
read-first: crates/marlin-org-workspace/tests/unit/document/contracts/standard_library.rs
contract: agent.plan.v1
resume-query: marlin-org-workspace standard agent contract plan ledger
next-action: continue from standard library validation receipts
#+end_src

* TODO Agent contract task
:PROPERTIES:
:CONTRACT_ORG: agent.task.v1
:END:

** Goal
Implement P0-P1 for Contract Org.

** Acceptance
- [ ] Standard contracts load from org/contracts.

** Progress
- [X] Contract documents are independent files.
- [ ] Contract templates are ready for downstream plans.

** Evidence
- standard_library.rs validates task receipts.

* Agent loop contract
:PROPERTIES:
:CONTRACT_ORG: agent.loop.v1
:END:

** Graph
Plan -> Task -> Memory.

** Strategy
static-org-contract-validation

** Budget
max-node-executions = 3

** Evidence
- graph execution receipts stay replayable.

* Agent memory contract
:PROPERTIES:
:CONTRACT_ORG: agent.memory.v1
:ID: agent-contract-memory-20260615
:MEMORY_KIND: contract-validation-record
:SOURCE_REF: crates/marlin-org-workspace/tests/unit/document/contracts/standard_library.rs
:RECALL_QUERY: standard contract workspace memory candidate validation
:SALIENCE: medium
:END:

** Claim
Standard contract validation should cover agent-facing memory candidate records.

** Source
Org workspace contract receipts.

** Recall
- Recall when testing standard agent contract loading across plan, task, loop, and memory records.

** Evidence
- The combined fixture validates all standard contract references.

** Trust
internal

** Retention
- Keep for project lifecycle while standard Contract Org examples are maintained.

* Agent topology contract
:PROPERTIES:
:CONTRACT_ORG: agent.topology.v1
:TOPOLOGY_ID: topology.standard-contract-workspace
:PROJECT_ID: marlin-agent-core
:TOPOLOGY_SCOPE: project-overview
:TOPOLOGY_NODE_KIND: project
:TOPOLOGY_EDGE_KIND: imports-project
:SOURCE_ANCHOR: crates/marlin-org-workspace/tests/unit/document/contracts/standard_library.rs
:END:

** Overview
The imported project topology gives agents a coarse route map before they open
detailed Org memory or session facts.

** Navigation
- project imports standard contract workspace.
- root session opens from the project topology.
- child session branches from a content anchor.

** Visibility Boundaries
- Sibling sessions stay topology-visible but transcript-hidden by default.
- Detailed evidence remains in the linked Org fact layer.
"#,
    );

    let mut candidates = vec![work_document.clone()];
    candidates.extend(contract_documents);
    let discovered = OrgDocumentLoader::discover_contract_documents(&candidates);
    let workspace = OrgDocumentLoader::load_workspace_with_contracts(&work_document, &discovered)
        .expect("workspace loads with standard contracts");

    assert_eq!(discovered.len(), 5);
    assert!(workspace.contract_resolutions.diagnostics.is_empty());
    assert!(workspace.contract_validations.diagnostics.is_empty());
    assert_eq!(
        resolved_contract_ids(&workspace),
        BTreeSet::from([
            "agent.loop.v1".to_owned(),
            "agent.memory.v1".to_owned(),
            "agent.plan.v1".to_owned(),
            "agent.task.v1".to_owned(),
            "agent.topology.v1".to_owned(),
        ])
    );
    assert_eq!(workspace.contract_validations.receipts.len(), 64);
    let failed_receipts = workspace
        .contract_validations
        .receipts
        .iter()
        .filter(|receipt| receipt.status != OrgContractValidationStatus::Passed)
        .map(|receipt| {
            (
                receipt.contract_id.as_str(),
                receipt.assertion_id.as_str(),
                receipt.severity.clone(),
                receipt.message.as_deref(),
                receipt.matched_nodes.len(),
            )
        })
        .collect::<Vec<_>>();
    assert!(failed_receipts.is_empty(), "{failed_receipts:#?}");
}

#[test]
fn org_document_loader_validates_standard_agent_contract_examples() {
    assert_standard_contract_example_passes(
        "doc:agent-plan-v1-example",
        include_str!("../../../../../../org/contracts/examples/agent.plan.v1.example.org"),
        "agent.plan.v1",
        30,
    );
    assert_standard_contract_example_passes(
        "doc:agent-task-v1-example",
        include_str!("../../../../../../org/contracts/examples/agent.task.v1.example.org"),
        "agent.task.v1",
        7,
    );
    assert_standard_contract_example_passes(
        "doc:agent-loop-v1-example",
        include_str!("../../../../../../org/contracts/examples/agent.loop.v1.example.org"),
        "agent.loop.v1",
        4,
    );
    assert_standard_contract_example_passes(
        "doc:agent-memory-v1-example",
        include_str!("../../../../../../org/contracts/examples/agent.memory.v1.example.org"),
        "agent.memory.v1",
        14,
    );
    assert_standard_contract_example_passes(
        "doc:agent-topology-v1-example",
        include_str!("../../../../../../org/contracts/examples/agent.topology.v1.example.org"),
        "agent.topology.v1",
        9,
    );
}

#[test]
fn org_document_loader_reports_standard_agent_memory_contract_failures() {
    let contract_documents = standard_agent_contract_documents();
    let memory_document = OrgDocument::new(
        "doc:memory-contract-failure",
        r#"* Agent memory candidate
:PROPERTIES:
:CONTRACT_ORG: agent.memory.v1
:ID: memory-contract-failure
:MEMORY_KIND: implementation-note
:SOURCE_REF: docs/20-workspace/20.40-org-memory-backend.org
:RECALL_QUERY: memory backend contract failure test
:SALIENCE: medium
:END:

** Claim
Memory contracts should report missing retention policy.

** Source
Developer review note.

** Recall
- Recall when testing missing memory retention.

** Evidence
- standard contract tests report failure receipts.

** Trust
internal
"#,
    );

    let workspace =
        OrgDocumentLoader::load_workspace_with_contracts(&memory_document, &contract_documents)
            .expect("workspace loads with standard memory contract");

    assert!(workspace.contract_resolutions.diagnostics.is_empty());
    let retention = workspace
        .contract_validations
        .receipts
        .iter()
        .find(|receipt| receipt.assertion_id == "memory.has-retention")
        .expect("retention receipt");

    assert_eq!(retention.status, OrgContractValidationStatus::Failed);
    assert!(
        workspace
            .contract_validations
            .receipts
            .iter()
            .filter(|receipt| receipt.assertion_id.starts_with("memory."))
            .any(|receipt| {
                receipt.assertion_id == "memory.has-source"
                    && receipt.status == OrgContractValidationStatus::Passed
            })
    );
}

#[test]
fn org_document_loader_reports_standard_agent_task_template_failures() {
    let contract_documents = standard_agent_contract_documents();
    let task_document = OrgDocument::new(
        "doc:task-template-failure",
        r#"* TODO Agent task candidate
:PROPERTIES:
:CONTRACT_ORG: agent.task.v1
:END:

** Goal
Make task contracts enforce a useful shape.

** Acceptance
Acceptance exists but has no checklist.

** Progress
- [ ] Add checklist validation.

** Evidence
- standard contract tests report failure receipts.
"#,
    );

    let workspace =
        OrgDocumentLoader::load_workspace_with_contracts(&task_document, &contract_documents)
            .expect("workspace loads with standard task contract");

    assert!(workspace.contract_resolutions.diagnostics.is_empty());
    let acceptance_checklist = workspace
        .contract_validations
        .receipts
        .iter()
        .find(|receipt| receipt.assertion_id == "task.acceptance-has-checklist")
        .expect("acceptance checklist receipt");
    let workflow = workspace
        .contract_validations
        .receipts
        .iter()
        .find(|receipt| receipt.assertion_id == "task.has-workflow-state")
        .expect("workflow receipt");

    assert_eq!(
        acceptance_checklist.status,
        OrgContractValidationStatus::Failed,
        "{acceptance_checklist:#?}"
    );
    assert_eq!(workflow.status, OrgContractValidationStatus::Passed);
}

fn contract_ids(workspace: &OrgDocumentWorkspace) -> BTreeSet<String> {
    workspace
        .contracts
        .contracts
        .iter()
        .map(|contract| contract.id.as_str().to_owned())
        .collect()
}

fn assertion_ids(workspace: &OrgDocumentWorkspace) -> BTreeSet<String> {
    workspace
        .contracts
        .contracts
        .iter()
        .flat_map(|contract| contract.assertions.iter())
        .map(|assertion| assertion.id.clone())
        .collect()
}

fn resolved_contract_ids(workspace: &OrgDocumentWorkspace) -> BTreeSet<String> {
    workspace
        .contract_resolutions
        .references
        .iter()
        .filter_map(|resolution| resolution.resolved_contract_id.as_ref())
        .map(|contract_id| contract_id.as_str().to_owned())
        .collect()
}

fn standard_document_ids() -> BTreeSet<String> {
    standard_agent_contract_documents()
        .iter()
        .map(|document| document.id.as_str().to_owned())
        .collect()
}

fn assert_standard_contract_example_passes(
    document_id: &'static str,
    source: &'static str,
    contract_id: &'static str,
    expected_receipts: usize,
) {
    let contract_documents = standard_agent_contract_documents();
    let example_document = OrgDocument::new(document_id, source);

    let workspace =
        OrgDocumentLoader::load_workspace_with_contracts(&example_document, &contract_documents)
            .expect("workspace loads with standard contract and example");

    assert!(workspace.contract_resolutions.diagnostics.is_empty());
    assert_eq!(
        resolved_contract_ids(&workspace),
        BTreeSet::from([contract_id.to_owned()])
    );
    assert_eq!(
        workspace.contract_validations.receipts.len(),
        expected_receipts
    );

    let failed_receipts = workspace
        .contract_validations
        .receipts
        .iter()
        .filter(|receipt| receipt.status != OrgContractValidationStatus::Passed)
        .map(|receipt| {
            (
                receipt.contract_id.as_str(),
                receipt.assertion_id.as_str(),
                receipt.severity.clone(),
                receipt.message.as_deref(),
                receipt.matched_nodes.len(),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        failed_receipts.is_empty(),
        "{document_id}: {failed_receipts:#?}"
    );
}
