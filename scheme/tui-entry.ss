;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Linked Gerbil VM entry. The selected publication remains owned by init.ss.
(import :gerbil-scheme-rust/scheme/runtime
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports)
        :marlin-agent-core/c-abi/ui)
(export main)

(def (main . _) (void))
(def (marlin-tui-publication _) marlin-agent-core.init.exports)

(begin-foreign
  (c-define (ui-publication root) (int64) int64
    "marlin_ui_publication" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
      marlin-agent-core/tui-entry#marlin-tui-publication)))
