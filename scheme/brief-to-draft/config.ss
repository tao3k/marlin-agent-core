;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .call)
        (only-in :clan/poo/type validate)
        "./types.ss" "./objects.ss")
(export BriefToDraftScope validate-settings propose)

(def BriefToDraftScope
  (validate BriefToDraftScopeContract BriefToDraftOperations))

(def (validate-settings input)
  (.call BriefToDraftScope validate-settings input))

(def (propose input)
  (.call BriefToDraftScope propose input))
