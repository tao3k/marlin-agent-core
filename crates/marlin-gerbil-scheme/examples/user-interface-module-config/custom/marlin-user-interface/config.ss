;;; -*- Gerbil -*-
;;; Boundary: user-owned Marlin interface module body.
;;; Invariant: this file only loads downstream declarations.

(import :poo-flow/src/module-system/init-syntax)

(load! "custom/marlin-user-interface/profiles/workspace")
(load! "custom/marlin-user-interface/profiles/session")
(load! "custom/marlin-user-interface/profiles/loops")
(load! "custom/marlin-user-interface/cases/user-interface")
(load! "custom/marlin-user-interface/cases/funflow")
