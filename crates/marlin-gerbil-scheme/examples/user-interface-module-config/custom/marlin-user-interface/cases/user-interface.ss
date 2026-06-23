;;; -*- Gerbil -*-
;;; Boundary: downstream Marlin user-interface case selection.
;;; Invariant: policy execution still crosses Marlin's Rust-owned receipts.

(use-module marlin-user-interface
  +delivery-receipt
  +policy-projection
  +loop-governor-manifest
  (policy-pack . "user-interface-prefab-pack")
  (delivery-entrypoint . UserInterfaceDeliveryReceipt)
  (apply-entrypoint . UserInterfaceApply)
  (projection-entrypoint . UserInterfacePolicyProjection))
