;;; -*- Gerbil -*-
;;; Boundary: Marlin-owned kernel policy module body.
;;; Invariant: root init.ss decides whether this kernel policy pack is enabled.

package: config-interface/custom/marline-kernel

(import (only-in :config-interface/custom/marline-kernel/policies/loops/profiles/runtime-handoff
                 poo-flow-custom-module-runtime-handoff-module)
        (only-in :config-interface/custom/marline-kernel/policies/loops/profiles/policy-receipt-gate
                 poo-flow-custom-module-policy-receipt-gate-module)
        (only-in :config-interface/custom/marline-kernel/policies/loops/profiles/loop-contract
                 poo-flow-custom-module-loop-contract-module)
        (only-in :config-interface/custom/marline-kernel/policies/loops/profiles/failure-retry
                 poo-flow-custom-module-failure-retry-module)
        (only-in :config-interface/custom/marline-kernel/policies/loops/cases/runtime-handoff-llm
                 poo-flow-custom-module-runtime-handoff-llm-case)
        (only-in :config-interface/custom/marline-kernel/policies/loops/cases/policy-receipt-gate-llm
                 poo-flow-custom-module-policy-receipt-gate-llm-case)
        (only-in :config-interface/custom/marline-kernel/policies/loops/cases/loop-contract-llm
                 poo-flow-custom-module-loop-contract-llm-case)
        (only-in :config-interface/custom/marline-kernel/policies/loops/cases/failure-retry-llm
                 poo-flow-custom-module-failure-retry-llm-case))

(export poo-flow-custom-module-runtime-handoff-module
        poo-flow-custom-module-policy-receipt-gate-module
        poo-flow-custom-module-loop-contract-module
        poo-flow-custom-module-failure-retry-module
        poo-flow-custom-module-runtime-handoff-llm-case
        poo-flow-custom-module-policy-receipt-gate-llm-case
        poo-flow-custom-module-loop-contract-llm-case
        poo-flow-custom-module-failure-retry-llm-case)
