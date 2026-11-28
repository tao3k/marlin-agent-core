;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Declarative POO Testing Governance Profiles with Cedar forbid/permit rules.
;;;
;;; Search / Helm / ASP / Dusa testing profiles strictly deny all write and
;;; shell capabilities (fail-closed Cedar `forbid` overrides every `permit`).
(import (only-in :clan/poo/object .def .mix .o seal-object!))
(export TestingGovernancePolicy.
        ReadOnlySearchTestingGovernance.
        ReadOnlySearchTestingGovernanceValue)

(.def TestingGovernancePolicy.
  id: 'default-readonly-testing-governance
  write-allowed?: #f
  allowed-capabilities: '(search)
  denied-capabilities: '(shell write edit mutation)
  allowed-tools: '(workspace_search)
  denied-tools: '(bash)
  cedar-policies:
  '((permit-search-tool
     "permit(principal, action == Marlin::Action::\"tool.search\", resource);")
    (forbid-write-capability
     "forbid(principal, action == Marlin::Action::\"tool.write\", resource);")
    (forbid-shell-capability
     "forbid(principal, action == Marlin::Action::\"tool.shell\", resource);")
    (forbid-write-context
     "forbid(principal, action, resource) when { context.is_write == true };"))
  denial-message:
  "Governance policy DENIED this tool call (read-only search/Helm/ASP/Dusa testing forbids all write and shell capabilities). Use the unified read-only `workspace_search` tool instead for L0 `(brute-search ...)` / `(file-outline ...)` / `(read-file ...)`, L1 `(find-symbols ...)` / `(find-impact ...)` / `:gerbil-ascent`, and L2 Dusa `(solve-placement ...)` / `(search-choice-program ...)`.")

(def ReadOnlySearchTestingGovernance.
  (.mix (.o id: 'read-only-search-governance
            write-allowed?: #f
            allowed-capabilities: '(search)
            denied-capabilities: '(shell write edit mutation)
            allowed-tools: '(workspace_search)
            denied-tools: '(bash))
        TestingGovernancePolicy.))

(def ReadOnlySearchTestingGovernanceValue
  (seal-object! ReadOnlySearchTestingGovernance.))
