;;; -*- Gerbil -*-
;;; Boundary: downstream user-owned POO Flow init entrypoint.
;;; Invariant: this file only selects modules; Marlin owns furnished policy packs.

(import :poo-flow/src/module-system/root-profile)

(poo-flow!
 :workflow
 (funflow)
 (loop-engine)
 :sandbox
 (nono-sandbox +nono +doctor)
 :custom
 (marlin-user-interface "./custom/marlin-user-interface" +private +doctor))
