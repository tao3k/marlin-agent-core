;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Declarative modeline content and semantic tones. The host resolves live
;;; model, session and activity facts without a VM call on each frame.
(import (only-in :clan/poo/object .def .get .mix .o seal-object!)
        (only-in :std/list/list find)
        (only-in "../ui-elements/objects.ss" ui-bar ui-template ui-choice ui-size-binding ui-size-cells ui-size-subtract))
(export marlin-ui-modeline-nodes marlin-ui-modeline-nodes-for marlin-ui-modeline-bar-for)

(.def UiModelineBase.
  activity-separator: "│ "
  slots: '((modeline/left/brand "🐟 MARLIN" "🐟" accent accent 0 18 0)
           (modeline/left/model "  {value}  " "{value}" info info 0 0 3)
           (modeline/left/activity "🟡 Ready" "🟡" info warning 0 64 0)))
(.def UiModelineSession.
  (slots => append '((modeline/right/turns "⟳{value}  " "⟳{value}" info info 72 0 1)
                     (modeline/right/session " ◉ {value} " "◉{value}" muted muted 44 0 2)
                     (modeline/state/online "🟢 Online" "🟢" success success 0 64 0)
                     (modeline/state/offline "🔴 Offline" "🔴" warning warning 0 64 0)
                     (modeline/state/unknown "🟠 Unknown" "🟠" warning warning 0 64 0)
                     (modeline/state/runtime "⚙ Runtime" "⚙" info info 0 64 0)
                     (modeline/state/workspace "⌁ Workspace" "⌁" info info 0 64 0)
                     (modeline/state/recovery "🟠 Recovery" "🟠" warning warning 0 64 0)
                     (modeline/state/continuation "⚠ Continue blocked" "⚠" warning warning 0 64 0)
                     (modeline/state/history "◷ History" "◷" muted muted 0 64 0)
                     (modeline/state/working "◌ Working" "◌" info info 0 64 0)
                     (modeline/state/reading "⌁ Reading" "⌁" info info 0 64 0))))
(def modeline-catalog
  (seal-object! (.mix UiModelineSession. UiModelineBase.)))

(def pi-modeline-catalog
  (seal-object! (.o activity-separator: "· "
                    slots: '((modeline/left/brand "🐟 marlin" "🐟" info info 0 16 0)
                              (modeline/left/model "  {value}  " "{value}" accent accent 0 0 1)
                              (modeline/left/activity "● ready" "●" success warning 0 48 0)
                              (modeline/right/session " ◉ {value} " "◉{value}" muted muted 40 0 3)
                              (modeline/right/turns "⟳{value}  " "⟳{value}" info info 66 0 2)
                              (modeline/state/online "● online" "●" success success 0 48 0)
                              (modeline/state/offline "● offline" "●" warning warning 0 48 0)
                              (modeline/state/unknown "● unknown" "●" warning warning 0 48 0)
                              (modeline/state/runtime "● runtime" "●" info info 0 48 0)
                              (modeline/state/workspace "● workspace" "●" info info 0 48 0)
                              (modeline/state/recovery "● recovery" "●" warning warning 0 48 0)
                              (modeline/state/continuation "● blocked" "●" warning warning 0 48 0)
                              (modeline/state/history "● history" "●" muted muted 0 48 0)
                              (modeline/state/working "● working" "●" info info 0 48 0)
                              (modeline/state/reading "● reading" "●" info info 0 48 0)))))

(def (marlin-ui-modeline-nodes-for profile)
  (def catalog (case profile
                 ((doom-one) modeline-catalog)
                 ((pi) pi-modeline-catalog)
                 (else (error "unknown UI modeline profile"))))
  (map (lambda (entry)
         (let* ((identity-value (car entry))
                (activity? (or (eq? identity-value 'modeline/left/activity)
                               (string-prefix? "modeline/state/" (symbol->string identity-value))))
                (prefix (if activity? (.get catalog activity-separator) "")))
           (.o identity: identity-value label: (string-append prefix (cadr entry))
             compact: (string-append prefix (caddr entry)) tone: (cadddr entry)
             active-tone: (list-ref entry 4)
             min-width: (list-ref entry 5) compact-below: (list-ref entry 6)
             keep-priority: (list-ref entry 7))))
       (.get catalog slots)))

(def (marlin-ui-modeline-nodes)
  (marlin-ui-modeline-nodes-for 'doom-one))

;;; Turn the selected catalog into a generic flow. The host only supplies facts
;;; and values; ordering, colors, compact forms and retention stay here.
(def (marlin-ui-modeline-bar-for nodes)
  (let* ((slot (lambda (identity-value)
                 (or (find (lambda (entry) (eq? (.get entry identity) identity-value)) nodes)
                     (error "missing modeline entry" identity-value))))
         (width-fact (lambda (width-value)
                       (string->symbol (string-append "width-at-least-" (number->string width-value)))))
         (fixed (lambda (text-value tone-value background-value bold-value)
                  (ui-template background-value (list (list 'literal tone-value bold-value text-value)))))
         (compact (lambda (entry full-value short-value)
                    (ui-choice (list (list (width-fact (.get entry compact-below)) full-value)) short-value)))
         (dynamic (lambda (template-value value-name tone-value tail-value)
                    (let* ((parts (string-split template-value #\{))
                           (before (car parts))
                           (suffix (if (and (= (length parts) 2)
                                            (string-prefix? "value}" (cadr parts)))
                                     (cadr parts)
                                     (error "missing modeline value placeholder" template-value)))
                           (after (substring suffix 6 (string-length suffix)))
                           (value-span (if tail-value
                                         (list 'tail tone-value #f value-name
                                           (ui-size-subtract (ui-size-binding 'bar-right-budget)
                                             (ui-size-cells (+ (string-length before) (string-length after)))))
                                         (list 'binding tone-value #f value-name))))
                      (ui-template 'none (list (list 'literal tone-value #f before)
                                              value-span (list 'literal tone-value #f after))))))
         (activity (lambda (entry)
                     (let (tone-value (.get entry tone))
                       (compact entry
                         (fixed (string-append (.get entry label) " ") tone-value 'none #f)
                         (fixed (string-append (.get entry compact) " ") tone-value 'none #f)))))
         (activity-view
           (ui-choice
             (map (lambda (name-value)
                    (let* ((name (symbol->string name-value))
                           (entry (slot (string->symbol (string-append "modeline/state/" name)))))
                      (list (string->symbol (string-append "activity-" name)) (activity entry))))
                  '(online offline unknown runtime workspace recovery continuation history working reading))
             (activity (slot 'modeline/left/activity))))
         (segments
           (apply append
             (map (lambda (entry)
                    (let* ((id (.get entry identity)) (tone-value (.get entry tone))
                           (min-width (.get entry min-width)) (priority (.get entry keep-priority)))
                      (case id
                        ((modeline/left/brand)
                         (list (list 'left 0 0 #f
                                 (compact entry
                                   (fixed (string-append " " (.get entry label) " ") 'canvas tone-value #t)
                                   (fixed (string-append " " (.get entry compact) " ") 'canvas tone-value #t)))
                               (list 'left 0 0 #f (fixed "▸" tone-value 'none #f))))
                        ((modeline/left/activity) (list (list 'left 0 0 #f activity-view)))
                        ((modeline/left/model modeline/right/turns modeline/right/session)
                         (let* ((side (if (eq? id 'modeline/left/model) 'left 'right))
                                (value-name (case id ((modeline/left/model) 'model)
                                                   ((modeline/right/turns) 'turns) (else 'session)))
                                (tail-value (eq? id 'modeline/right/session))
                                (condition (and (eq? id 'modeline/right/turns) 'has-turns)))
                           (list (list side priority min-width condition
                                   (compact entry
                                     (dynamic (.get entry label) value-name tone-value tail-value)
                                     (dynamic (.get entry compact) value-name tone-value tail-value))))))
                        (else '())))) nodes))))
    (ui-bar 'bar 3 segments)))
