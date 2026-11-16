;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/mop Fun Object Type.)
        (only-in :clan/poo/type Exactly Record)
        (only-in :std/list/list duplicates length<=n?))
(export UiThemesScopeContract)

(def UiProfileIdentity (.cc Type. '.element? symbol?))
(def UiThemeProfiles
  (.cc Type. '.element?
    (lambda (profiles)
      (and (pair? profiles) (length<=n? profiles 8) (list? profiles)
           (andmap (lambda (entry)
                     (and (length<=n? entry 3) (list? entry) (= (length entry) 3)
                          (symbol? (car entry)) (string? (cadr entry))
                          (<= (string-length (cadr entry)) 256)))
                   profiles)
           (null? (duplicates (map car profiles) eq?))))))

(def UiThemesScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-themes)] selected: [Object]
               default-profile: [UiProfileIdentity] profiles: [UiThemeProfiles]
               select: [(Fun Object <- UiProfileIdentity)])
       identity: 'marlin/ui-themes))
