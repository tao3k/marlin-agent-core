;;; -*- Gerbil -*-
;;; Boundary: Marlin-owned kernel policy init entrypoint.
;;; Invariant: this file only selects module packs; policy bodies live in custom/.

(import :poo-flow/src/module-system/root-profile)

;;; Doom-style init shape: keep kernel policy selection compact and declarative.
(poo-flow!
 :workflow
 (loop-engine)
 :custom
 (marline-kernel "./custom/marline-kernel" +private +doctor))
