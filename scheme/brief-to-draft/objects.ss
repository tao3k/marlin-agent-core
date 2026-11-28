;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later


(import (only-in :clan/poo/object .o)
        (only-in "./projection.ss" project-settings project-proposal))
(export BriefToDraftOperations)

(def BriefToDraftOperations
  (.o identity: 'marlin/brief-to-draft
      program-entry: 'marlin-agent-core/brief-to-draft/entry
      program-runtime-imports: '()
      validate-settings: project-settings
      propose: project-proposal))
