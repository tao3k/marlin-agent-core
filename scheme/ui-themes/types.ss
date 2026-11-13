;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/mop Fun Object Type.)
        (only-in :clan/poo/type Exactly Record))
(export UiThemesScopeContract)

(def UiProfileIdentity (.cc Type. '.element? symbol?))
(def UiThemeProfiles
  (.cc Type. '.element?
    (lambda (profiles)
      (and (list? profiles) (<= 1 (length profiles) 8)
           (let loop ((rest profiles) (seen '()))
             (or (null? rest)
                 (let (entry (car rest))
                   (and (list? entry) (= (length entry) 3)
                        (symbol? (car entry)) (not (memq (car entry) seen))
                        (string? (cadr entry)) (<= (string-length (cadr entry)) 256)
                        (loop (cdr rest) (cons (car entry) seen))))))))))

(def UiThemesScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-themes)] selected: [Object]
               default-profile: [UiProfileIdentity] profiles: [UiThemeProfiles]
               select: [(Fun Object <- UiProfileIdentity)])
       identity: 'marlin/ui-themes))
