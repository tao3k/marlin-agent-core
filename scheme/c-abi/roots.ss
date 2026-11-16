;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Marlin owns its ABI entry points. The Rust host supplies the root service;
;;; no Rust SDK package participates in Scheme import or module selection.
(export marlin-rooted-call)
(extern root-procedure apply-procedure release-procedure)

;; The existing Cargo SDK C ABI accepts two borrowed roots. Root the Scheme
;; callback for the duration of that call; the SDK owns errors and result roots.
(def (marlin-rooted-call input procedure)
  (let (callback (root-procedure procedure))
    (with-unwind-protect
      (lambda () (apply-procedure callback input))
      (lambda () (release-procedure callback)))))

(begin-foreign
  (namespace ("marlin-agent-core/c-abi/roots#" root-procedure apply-procedure release-procedure))
  (c-declare "extern ___S64 gerbil_scheme_rust_object_root_raw(___SCMOBJ);\nextern ___S64 gerbil_scheme_rust_apply_rooted(___S64, ___S64);\nextern ___S32 gerbil_scheme_rust_root_release_raw(___S64);")
  (define root-procedure
    (c-lambda (scheme-object) int64 "gerbil_scheme_rust_object_root_raw"))
  (define apply-procedure
    (c-lambda (int64 int64) int64 "gerbil_scheme_rust_apply_rooted"))
  (define release-procedure
    (c-lambda (int64) int32 "gerbil_scheme_rust_root_release_raw")))
