;;; UI composition over host facts; no terminal operations or runtime effects.
(import (only-in :clan/poo/object .o .ref .slot? object? seal-object!)
        (only-in :yuanxu/poo-clos/types clos-instance?)
        "./dispatch.ss")
(export marlin-ui-workbench marlin-ui-focus-event marlin-ui-bounded-list?)

(def max-contributions 8)
(def max-extensions 16)
(def max-label-length 256)
(def reserved-identities '(transcript composer status commands))

(def (required-object value slots)
  (unless (and (object? value) (andmap (lambda (slot) (.slot? value slot)) slots))
    (error "invalid UI value")))

(def (bounded-label? value)
  (and (string? value) (<= (string-length value) max-label-length)))

(def (unique-identities? ids)
  (or (null? ids)
      (and (not (memq (car ids) (cdr ids))) (unique-identities? (cdr ids)))))

;;; Stop at the admission limit, including for improper and cyclic lists.
;;; A separate list? / length pass would traverse arbitrarily large input
;;; before the workbench could reject it.
(def (marlin-ui-bounded-list? values limit)
  (let loop ((rest values) (remaining limit))
    (cond ((null? rest) #t)
          ((or (zero? remaining) (not (pair? rest))) #f)
          (else (loop (cdr rest) (- remaining 1))))))

(def (contribution-node contribution)
  (required-object contribution '(identity label))
  (let ((identity-value (.ref contribution 'identity)) (label-value (.ref contribution 'label)))
    (unless (and (symbol? identity-value) (bounded-label? label-value)
                 (not (memq identity-value reserved-identities)))
      (error "invalid UI contribution"))
    (let (owned-label (string-copy label-value))
      (.o (:: @ UiStatusItem.) identity: identity-value kind: 'status-item label: owned-label))))

(def (checked-extension component)
  (required-object component '(identity kind label focusable))
  (unless (and (clos-instance? UiExtension. component)
               (symbol? (.ref component 'identity))
               (not (memq (.ref component 'identity) reserved-identities))
               (eq? (.ref component 'kind) 'extension)
               (bounded-label? (.ref component 'label))
               (boolean? (.ref component 'focusable)))
    (error "invalid UI extension component"))
  ;; Snapshot presentation and focus fields without sealing the contributor's
  ;; object. Its prototype remains in the dispatch precedence list.
  (let ((identity-value (.ref component 'identity))
        (label-value (string-copy (.ref component 'label)))
        (focusable-value (.ref component 'focusable)))
    (seal-object! (.o (:: @ component) identity: identity-value kind: 'extension
                      label: label-value focusable: focusable-value))))

;;; Host facts are display data only. Transcript and composer are named regions
;;; whose contents and editing authority remain with their existing owners.
(def (marlin-ui-workbench facts)
  (required-object facts '(model session contributions extensions))
  (let ((model-value (.ref facts 'model)) (session-value (.ref facts 'session))
        (contribution-values (.ref facts 'contributions))
        (extension-values (.ref facts 'extensions)))
    (unless (and (bounded-label? model-value) (bounded-label? session-value)
                 (marlin-ui-bounded-list? contribution-values max-contributions)
                 (marlin-ui-bounded-list? extension-values max-extensions))
      (error "invalid UI workbench facts"))
    (let* ((owned-model (string-copy model-value))
           (owned-session (string-copy session-value))
           (nodes (map contribution-node contribution-values))
           (extensions (map checked-extension extension-values))
           (ids (map (lambda (node) (.ref node 'identity)) (append nodes extensions)))
           (extension-focus (map (lambda (node) (.ref node 'identity))
                                 (filter (lambda (node) (.ref node 'focusable)) extensions))))
      (unless (unique-identities? ids)
        (error "duplicate UI contribution identity"))
      (.o (:: @ UiWorkbench.) identity: 'workbench kind: 'workbench
          model: owned-model session: owned-session
          focus-order: (append '(transcript composer commands) extension-focus)
          regions: (append (list (.o (:: @ UiTranscript.) identity: 'transcript kind: 'transcript)
                                 (.o (:: @ UiComposer.) identity: 'composer kind: 'composer)
                                 (.o (:: @ UiCommands.) identity: 'commands kind: 'commands))
                           extensions
                           (list (.o (:: @ UiStatus.) identity: 'status kind: 'status children: nodes)))))))

(def (marlin-ui-focus-event workbench)
  (let (order-value (.ref workbench 'focus-order))
    (.o (:: @ UiNextFocus.) order: order-value)))
