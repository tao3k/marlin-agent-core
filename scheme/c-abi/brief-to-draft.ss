;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Rooted typed values use the shared Gerbil Scheme Rust ABI.
(import :gerbil-scheme-rust/scheme/runtime
        (only-in :marlin-agent-core/brief-to-draft/objects BriefToDraftOperations)
        (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports))
(export main marlin-brief-to-draft-settings)

(def (main . _) (void))

(def (marlin-brief-to-draft-settings input)
  ((.get marlin-agent-core.init.exports Marlin brief-to-draft validate-settings) input))

(begin-foreign
  (c-define (validate-settings root) (int64) int64
    "marlin_brief_to_draft_validate_settings" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
      marlin-agent-core/c-abi/brief-to-draft#marlin-brief-to-draft-settings)))

(def (marlin-brief-to-draft-propose input)
  ((.get marlin-agent-core.init.exports Marlin brief-to-draft propose) input))

(begin-foreign
  (c-define (propose root) (int64) int64
    "marlin_brief_to_draft_propose" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
      marlin-agent-core/c-abi/brief-to-draft#marlin-brief-to-draft-propose)))
