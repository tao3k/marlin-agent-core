;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Type contracts for Marlin's Modern Helm (`scheme/ui-helm`).
(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/type Exactly Fun List Object Record String Symbol Type.))
(export HelmScopeContract
        HelmVector
        HelmInteger
        HelmAny)

(def HelmVector (.cc Type. '.element? vector?))
(def HelmInteger (.cc Type. '.element? integer?))
(def HelmAny (.cc Type. '.element? (lambda (_) #t)))

(def HelmScopeContract
  (.cc (Record identity: [(Exactly 'marlin/ui-helm)]
               default-sources: [(List Object)]
               default-actions: [(List Object)]
               open-session: [(Fun Object <-)]
               parse-input: [(Fun Object <- String)]
               update-input: [(Fun Object <- Object String)]
               compile-plan-vector: [(Fun HelmVector <- String)]
               apply-results: [(Fun Object <- Object HelmInteger HelmAny (List Object))]
               move-selection: [(Fun Object <- Object HelmInteger)]
               toggle-mark: [(Fun Object <- Object)]
               next-source: [(Fun Object <- Object)]
               toggle-preview: [(Fun Object <- Object)]
               toggle-actions: [(Fun Object <- Object)]
               dispatch-action: [(Fun Object <- Object Symbol)]
               render-lines: [(Fun (List String) <- Object)]
               project-ui-elements: [(Fun HelmVector <- Object)])
       identity: 'marlin/ui-helm))
