;;; -*- Gerbil -*-
;;; Boundary: Marlin-owned kernel policy module body.
;;; Invariant: root init.ss decides whether this kernel policy pack is enabled.

(import :poo-flow/src/module-system/init-syntax)

(load! "custom/marline-kernel/policies/loops/profiles/runtime-handoff")
(load! "custom/marline-kernel/policies/loops/profiles/policy-receipt-gate")
(load! "custom/marline-kernel/policies/loops/profiles/loop-contract")
(load! "custom/marline-kernel/policies/loops/profiles/failure-retry")
(load! "custom/marline-kernel/policies/loops/cases/runtime-handoff-llm")
(load! "custom/marline-kernel/policies/loops/cases/policy-receipt-gate-llm")
(load! "custom/marline-kernel/policies/loops/cases/loop-contract-llm")
(load! "custom/marline-kernel/policies/loops/cases/failure-retry-llm")
