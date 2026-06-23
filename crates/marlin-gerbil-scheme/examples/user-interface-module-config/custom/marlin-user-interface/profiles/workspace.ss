;;; -*- Gerbil -*-
;;; Boundary: downstream Marlin workspace selection.
;;; Invariant: use-module stores data; Marlin adapters build runtime receipts.

(use-module marlin-user-interface
  (workspace-root . "user-interface-workspace")
  (interface-file . "interface.org")
  (state-file . "state/worker-state.org")
  (model-profile . "interactive")
  (hook-id . "runtime-catalog-user-interface-hook")
  (hook-action . "register")
  (hook-owner . "user-interface-worker")
  (continuation-profile . "user-interface-loop-continuation"))
