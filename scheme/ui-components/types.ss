;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/mop Fun Object Type.)
        (only-in :clan/poo/type Exactly Record)
        (only-in "./commands.ss" marlin-ui-command-entries?))
(export UiComponentsScopeContract)

(def UiCommandEntries (.cc Type. '.element? marlin-ui-command-entries?))
(def UiTransientEntries (.cc Type. '.element? vector?))
(def UiCommandQuery (.cc Type. '.element? (lambda (value) (and (string? value) (<= (string-length value) 32)))))
(def UiCommandHistory (.cc Type. '.element? (lambda (value) (and (list? value) (<= (length value) 256) (andmap (lambda (name) (and (string? name) (<= (string-length name) 32))) value)))))
(def UiCommandNames (.cc Type. '.element? (lambda (value) (and (list? value) (<= (length value) 256) (andmap string? value)))))

(def UiComponentsScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-components)]
               components: [Object]
               contribution-node: [(Fun Object <- Object)]
               modeline-node: [(Fun Object <- Object)]
               extension-node: [(Fun Object <- Object)]
               command-catalog: [Object]
               command-entries: [(Fun UiCommandEntries <- Object)]
               command-candidates: [(Fun UiCommandNames <- UiCommandEntries UiCommandQuery UiCommandHistory)]
               command-extensions: [(Fun UiCommandEntries <-)]
               transient-entries: [(Fun UiTransientEntries <-)]
               browse-command: [(Fun Object <- UiCommandEntries Object)])
       identity: 'marlin/ui-components))
