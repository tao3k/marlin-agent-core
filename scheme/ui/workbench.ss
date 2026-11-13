;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; UI composition over host facts; no terminal operations or runtime effects.
(import (only-in :clan/poo/object .o .ref)
        (only-in "../ui-components/bounds.ss" marlin-ui-max-contributions marlin-ui-max-extensions
                 marlin-ui-bounded-label? marlin-ui-bounded-list? marlin-ui-required-object)
        "./dispatch.ss")
(export make-marlin-ui-workbench make-marlin-ui-profile-workbench
        marlin-ui-focus-event marlin-ui-previous-focus-event)

(def (unique-identities? ids)
  (or (null? ids)
      (and (not (memq (car ids) (cdr ids))) (unique-identities? (cdr ids)))))

;;; Host facts are display data only. Transcript and composer are named regions
;;; whose contents and editing authority remain with their existing owners.
(def (make-marlin-ui-profile-workbench components modeline transcript themes layout)
  (let* ((types (.ref components 'components))
        (contribution-node (.ref components 'contribution-node))
        (modeline-node (.ref components 'modeline-node))
        (extension-node (.ref components 'extension-node))
        (buffer-identities ((.ref components 'buffer-identities)))
        (command-entries (.ref components 'command-entries))
        (command-catalog
          ((.ref components 'command-catalog-with-profiles)
           ((.ref components 'command-catalog-with-extensions)
            (.ref components 'command-catalog) ((.ref components 'command-extensions)))
           (.ref themes 'profiles)))
        (modeline-nodes-for (.ref modeline 'nodes-for))
        (modeline-bar-for (.ref modeline 'bar-for))
        (layout-for (.ref layout 'policy-for))
        (transcript-for (.ref transcript 'select))
        (theme-for (.ref themes 'select)))
  (lambda (facts profile)
    (marlin-ui-required-object facts '(model session contributions extensions))
    (let ((model-value (.ref facts 'model)) (session-value (.ref facts 'session))
        (contribution-values (.ref facts 'contributions))
        (extension-values (.ref facts 'extensions)))
    (unless (and (marlin-ui-bounded-label? model-value) (marlin-ui-bounded-label? session-value)
                 (marlin-ui-bounded-list? contribution-values marlin-ui-max-contributions)
                 (marlin-ui-bounded-list? extension-values marlin-ui-max-extensions))
      (error "invalid UI workbench facts"))
    (let* ((owned-model (string-copy model-value))
           (owned-session (string-copy session-value))
           (modeline-values (modeline-nodes-for profile))
           (nodes (append (map modeline-node modeline-values)
                          (map contribution-node contribution-values)))
           (host-extensions (map extension-node extension-values))
           (extensions (append ((.ref components 'static-buffer-nodes))
                               host-extensions))
           (ids (map (lambda (node) (.ref node 'identity)) (append nodes extensions)))
           (extension-focus (map (lambda (node) (.ref node 'identity))
                                 (filter (lambda (node) (.ref node 'focusable)) extensions))))
      (unless (and (unique-identities? ids)
                   (not (ormap (lambda (node)
                                 (memq (.ref node 'identity) buffer-identities))
                               host-extensions)))
        (error "duplicate UI contribution identity"))
      (.o (:: @ (.ref types 'workbench)) identity: 'workbench kind: 'workbench
          model: owned-model session: owned-session geometry-policy: (layout-for (modeline-bar-for modeline-values))
          transcript-policy: (transcript-for profile)
          theme-palette: (theme-for profile)
          focus-order: (append '(transcript composer commands) extension-focus)
          regions: (append (list (.o (:: @ (.ref types 'transcript)) identity: 'transcript kind: 'transcript)
                                 (.o (:: @ (.ref types 'composer)) identity: 'composer kind: 'composer)
                                 (.o (:: @ (.ref types 'commands)) identity: 'commands kind: 'commands
                                     entries: (command-entries command-catalog)))
                           extensions
                           (list (.o (:: @ (.ref types 'status)) identity: 'status kind: 'status children: nodes)))))))))

(def (make-marlin-ui-workbench components modeline transcript themes layout)
  (def build (make-marlin-ui-profile-workbench components modeline transcript themes layout))
  (lambda (facts) (build facts (.ref themes 'default-profile))))

(def (marlin-ui-focus-event workbench)
  (let (order-value (.ref workbench 'focus-order))
    (.o (:: @ UiNextFocus.) order: order-value)))

(def (marlin-ui-previous-focus-event workbench)
  (let (order-value (.ref workbench 'focus-order))
    (.o (:: @ UiPreviousFocus.) order: order-value)))
