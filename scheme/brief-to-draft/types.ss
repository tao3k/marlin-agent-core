;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/mop Fun Type.)
        (only-in :clan/poo/type Exactly Record))
(export BriefToDraftScopeContract)

(def (vector-row? row width)
  (and (vector? row) (= (vector-length row) width)))

(def SettingsInput (.cc Type. '.element? (lambda (row) (vector-row? row 4))))
(def SettingsProjection (.cc Type. '.element? (lambda (row) (vector-row? row 3))))
(def ProposalInput (.cc Type. '.element? (lambda (row) (vector-row? row 3))))
(def ProposalResult (.cc Type. '.element? (lambda (row) (or (not row) (vector-row? row 3)))))
(def BriefToDraftScopeContract
  (.cc (Record identity: [(Exactly 'marlin/brief-to-draft)]
               validate-settings: [(Fun SettingsProjection <- SettingsInput)]
               propose: [(Fun ProposalResult <- ProposalInput)])
       identity: 'marlin/brief-to-draft))
