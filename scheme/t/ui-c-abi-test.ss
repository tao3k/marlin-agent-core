;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import :std/test
        (only-in :clan/poo/object .get .o)
        (only-in :gerbil-scheme-rust/scheme/runtime gerbil-rs-root-object)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports)
        (only-in :tao3k/yuanxu/poo-clos/generic-syntax .defgeneric .defmethod)
        (only-in :tao3k/yuanxu/poo-clos/method-bundle-syntax .defmethod-bundle)
        (only-in :marlin-agent-core/t/ui-c-abi-driver ui-test-open ui-test-project ui-test-dispatch))
(export ui-c-abi-test)
(extern namespace: gerbil-scheme-rust/scheme/runtime
        gerbil-rs-rooted-value-ref gerbil-rs-rooted-value-release! gerbil-rs-call-error)

(def ui (.get marlin-agent-core.init.exports Marlin ui))
(def selection marlin-agent-core.init.exports)
(def (check-project)
  (let* ((input (gerbil-rs-root-object (vector selection #("model" "session" #() #()) #())))
         (opened (ui-test-open input))
         (output (if (positive? opened) (ui-test-project opened)
                   (error "UI open failed" gerbil-rs-call-error))))
    (with-unwind-protect
      (lambda ()
        (unless (positive? output) (error "UI project failed" gerbil-rs-call-error))
        (check (positive? output) => #t)
        (check (not (= input output)) => #t)
        (let (value (gerbil-rs-rooted-value-ref output))
          (check-equal? (vector-ref value 0) "model")
          (check-equal? (vector-length (vector-ref value 2)) 4)
          (check-equal? (vector-ref value 3) 'composer))
        (check (vector? (gerbil-rs-rooted-value-ref input)) => #t))
      (lambda ()
        (gerbil-rs-rooted-value-release! input)
        (when (positive? opened) (gerbil-rs-rooted-value-release! opened))
        (when (positive? output) (gerbil-rs-rooted-value-release! output))))))

(def (check-dispatch)
  (let* ((request (gerbil-rs-root-object (vector selection #("model" "session" #() #()) #())))
         (opened (ui-test-open request))
         (input (gerbil-rs-root-object (vector (gerbil-rs-rooted-value-ref opened) 'next-focus)))
         (output (ui-test-dispatch input))
         (result (gerbil-rs-rooted-value-ref output))
         (successor (gerbil-rs-root-object (vector-ref result 0)))
         (old-view (ui-test-project opened))
         (new-view (ui-test-project successor)))
    (with-unwind-protect
      (lambda ()
        (check (positive? output) => #t)
        (check (not (= input output)) => #t)
        (check-equal? (vector-ref result 1) '#(focus commands))
        (check-equal? (vector-ref (gerbil-rs-rooted-value-ref old-view) 3) 'composer)
        (check-equal? (vector-ref (gerbil-rs-rooted-value-ref new-view) 3) 'commands)
        (check (vector? (gerbil-rs-rooted-value-ref input)) => #t))
      (lambda ()
        (gerbil-rs-rooted-value-release! input)
        (gerbil-rs-rooted-value-release! request)
        (gerbil-rs-rooted-value-release! opened)
        (gerbil-rs-rooted-value-release! successor)
        (gerbil-rs-rooted-value-release! old-view)
        (gerbil-rs-rooted-value-release! new-view)
        (when (positive? output) (gerbil-rs-rooted-value-release! output))))))

(def (check-submit-and-unknown)
  (let* ((request (gerbil-rs-root-object (vector selection #("model" "session" #() #()) #())))
         (opened (ui-test-open request))
         (state (gerbil-rs-rooted-value-ref opened))
         (submit (gerbil-rs-root-object (vector state 'submit)))
         (unknown (gerbil-rs-root-object (vector state 'unknown)))
         (output (ui-test-dispatch submit)))
    (with-unwind-protect
      (lambda ()
        (check (positive? output) => #t)
        (check-equal? (vector-ref (gerbil-rs-rooted-value-ref output) 1) '#(submit-draft))
        (check-equal? (ui-test-dispatch unknown) 0)
        (check (vector? (gerbil-rs-rooted-value-ref unknown)) => #t))
      (lambda ()
        (gerbil-rs-rooted-value-release! request)
        (gerbil-rs-rooted-value-release! opened)
        (gerbil-rs-rooted-value-release! submit)
        (gerbil-rs-rooted-value-release! unknown)
        (when (positive? output) (gerbil-rs-rooted-value-release! output))))))

(def (dispatch-value state kind)
  (let* ((request (gerbil-rs-root-object (vector state kind)))
         (result (ui-test-dispatch request)))
    (with-unwind-protect
      (lambda ()
        (unless (positive? result) (error "UI dispatch failed" gerbil-rs-call-error))
        (gerbil-rs-rooted-value-ref result))
      (lambda ()
        (gerbil-rs-rooted-value-release! request)
        (when (positive? result) (gerbil-rs-rooted-value-release! result))))))

(def (check-extension-method)
  (let* ((extension-type (.get ui components extension))
         (event-type (.get ui events submit))
         (extension (.o (:: @ extension-type) identity: 'extension/editor kind: 'extension
                        label: "Editor" focusable: #t)))
    (.defgeneric extension-submit (component event) (protocol (.get ui event-protocol)))
    (.defmethod-bundle ExtensionSubmitMethods (.get ui event-protocol)
      (.defmethod (extension-submit (component (prototype extension))
                                    (event (prototype event-type)))
        (.o kind: 'focus identity: 'composer)))
    (let* ((request (gerbil-rs-root-object
                      (vector selection (vector "model" "session" #() (vector extension))
                              (vector ExtensionSubmitMethods))))
           (opened (ui-test-open request)))
      (with-unwind-protect
        (lambda ()
          (unless (positive? opened) (error "extension UI open failed" gerbil-rs-call-error))
          (let* ((state (gerbil-rs-rooted-value-ref opened))
                 (first (dispatch-value state 'next-focus))
                 (second (dispatch-value (vector-ref first 0) 'next-focus))
                 (handled (dispatch-value (vector-ref second 0) 'submit)))
            (check-equal? (vector-ref first 1) '#(focus commands))
            (check-equal? (vector-ref second 1) '#(focus extension/editor))
            (check-equal? (vector-ref handled 1) '#(focus composer))
            ;; A second session without the bundle must not inherit methods.
            (let* ((plain-request (gerbil-rs-root-object
                                    (vector selection (vector "model" "session" #() (vector extension)) #())))
                   (plain-open (ui-test-open plain-request))
                   (plain-first (dispatch-value (gerbil-rs-rooted-value-ref plain-open) 'next-focus))
                   (plain-second (dispatch-value (vector-ref plain-first 0) 'next-focus))
                   (plain-submit (gerbil-rs-root-object (vector (vector-ref plain-second 0) 'submit))))
              (with-unwind-protect
                (lambda () (check-equal? (ui-test-dispatch plain-submit) 0))
                (lambda ()
                  (gerbil-rs-rooted-value-release! plain-request)
                  (gerbil-rs-rooted-value-release! plain-open)
                  (gerbil-rs-rooted-value-release! plain-submit))))))
        (lambda ()
          (gerbil-rs-rooted-value-release! request)
          (when (positive? opened) (gerbil-rs-rooted-value-release! opened)))))))

(def ui-c-abi-test
  (test-suite "Marlin rooted UI ABI"
    (test-case "project selected scope and preserve the borrowed input"
      (check-project))
    (test-case "dispatch against selected workbench and preserve the borrowed input"
      (check-dispatch))
    (test-case "submit is an intent and unknown events fail closed"
      (check-submit-and-unknown))
    (test-case "extension method specializes the session dispatch"
      (check-extension-method))
    (test-case "invalid root fails closed"
      (check-equal? (ui-test-open 0) 0)
      (check-equal? (ui-test-project 0) 0)
      (check-equal? (ui-test-dispatch 0) 0))
    (test-case "invalid method bundle fails session creation"
      (let (input (gerbil-rs-root-object (vector selection #("model" "session" #() #()) '#(bad))))
        (with-unwind-protect
          (lambda () (check-equal? (ui-test-open input) 0))
          (lambda () (gerbil-rs-rooted-value-release! input)))))
    (test-case "a UI child scope is not a selected publication"
      (let (input (gerbil-rs-root-object (vector ui #("model" "session" #() #()) #())))
        (with-unwind-protect
          (lambda () (check-equal? (ui-test-open input) 0))
          (lambda () (gerbil-rs-rooted-value-release! input)))))
    (test-case "invalid request retains the borrowed root"
      (let (input (gerbil-rs-root-object '#(invalid)))
        (with-unwind-protect
          (lambda ()
            (check-equal? (ui-test-project input) 0)
            (check-equal? (gerbil-rs-rooted-value-ref input) '#(invalid)))
          (lambda () (gerbil-rs-rooted-value-release! input)))))))
