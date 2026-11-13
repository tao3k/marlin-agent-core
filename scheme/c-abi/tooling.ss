;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Rooted Scheme values only. Rust validates the shape and owns provider JSON.
(import :gerbil-scheme-rust/scheme/runtime
        (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports))
(export marlin-tooling-project)

(def (marlin-tooling-project _)
  ((.get (.get marlin-agent-core.init.exports Marlin tooling) project)))

(begin-foreign
  (c-define (tooling-project root) (int64) int64
    "marlin_tooling_project" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
      marlin-agent-core/c-abi/tooling#marlin-tooling-project)))
