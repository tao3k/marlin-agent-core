;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Gerbil Scheme regression tests for scheme/ui-workspaces (Doom persp-mode
;;; workspace & buffer isolation + declarative set-popup-rule! popup manager).
(import :std/test
        (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports))

(export ui-workspaces-test)

(def ui-ws (.get marlin-agent-core.init.exports Marlin ui-workspaces))

(def ui-workspaces-test
  (test-suite "Marlin Scheme ui-workspaces (persp-mode isolation & Doom popup rules)"

    (test-case "Workspaces isolate open buffers and restore exact layout on switch"
      (let* ((s0 ((.get ui-ws make-state)))
             ;; Open *Sessions* in main workspace.
             (s1 ((.get ui-ws open-buffer) s0 "*Sessions*"))
             ;; Create a separate project-scoped workspace "review".
             (s2 ((.get ui-ws create-workspace) s1 "review" "Review" "/repo/marlin-ui"))
             (s3 ((.get ui-ws open-buffer) s2 "*Packages*"))
             (review-bufs ((.get ui-ws workspace-buffers) s3))
             (all-bufs ((.get ui-ws all-workspace-buffers) s3))
             ;; Switch back to main -> *Packages* must NOT leak into main's buffer list!
             (s4 ((.get ui-ws switch-workspace) s3 "main"))
             (main-bufs ((.get ui-ws workspace-buffers) s4)))
        (check-equal? (and (member "*Packages*" review-bufs) #t) #t)
        (check-equal? (member "*Sessions*" review-bufs) #f)
        (check-equal? (and (member "*Sessions*" main-bufs) #t) #t)
        (check-equal? (member "*Packages*" main-bufs) #f)
        (check-equal? (and (member "*Sessions*" all-bufs) (member "*Packages*" all-bufs) #t) #t)
        ;; Buffer candidate filtering isolates by active workspace unless include-all? is #t.
        (check-equal?
          ((.get ui-ws isolate-buffer-candidates) s4 '("*Dashboard*" "*Sessions*" "*Packages*") #f)
          '("*Dashboard*" "*Sessions*"))
        (check-equal?
          ((.get ui-ws isolate-buffer-candidates) s4 '("*Dashboard*" "*Sessions*" "*Packages*") #t)
          '("*Dashboard*" "*Sessions*" "*Packages*"))))

    (test-case "Popup rules route auxiliary buffers into popup drawer without disrupting main layout"
      (let* ((s0 ((.get ui-ws make-state)))
             (s1 ((.get ui-ws open-buffer) s0 "*conversation*"))
             ;; Opening *Help* matches DefaultPopupRules (bottom 0.36) and leaves main buffer untouched.
             (s2 ((.get ui-ws open-buffer) s1 "*Help*"))
             (proj2 ((.get ui-ws project-state) s2))
             (popup-vec (vector-ref proj2 3))
             ;; Dismissing popup restores clean state; raising promotes *Models* into main workspace.
             (s3 ((.get ui-ws dismiss-popup) s2))
             (s4 ((.get ui-ws open-buffer) s3 "*Models*"))
             (s5 ((.get ui-ws toggle-popup) s4))
             (s6 ((.get ui-ws toggle-popup) s5))
             (s7 ((.get ui-ws raise-popup) s6)))
        (check-equal? (vector-ref proj2 0) 'marlin-workspace-state)
        (check-equal? (vector-ref popup-vec 1) "*Help*")
        (check-equal? (vector-ref popup-vec 2) 'bottom)
        ;; *Help* did NOT displace *conversation* from main workspace buffer list!
        (check-equal? (member "*Help*" ((.get ui-ws workspace-buffers) s2)) #f)
        ;; Dismiss cleared active popup.
        (check-equal? (vector-ref ((.get ui-ws project-state) s3) 3) #f)
        ;; Toggle closed and reopened session-TTL *Models* popup on right side.
        (check-equal? (vector-ref ((.get ui-ws project-state) s5) 3) #f)
        (check-equal? (vector-ref (vector-ref ((.get ui-ws project-state) s6) 3) 1) "*Models*")
        (check-equal? (vector-ref (vector-ref ((.get ui-ws project-state) s6) 3) 2) 'right)
        ;; Raise promoted *Models* into main workspace buffers and cleared popup drawer.
        (check-equal? (vector-ref ((.get ui-ws project-state) s7) 3) #f)
        (check-equal? (and (member "*Models*" ((.get ui-ws workspace-buffers) s7)) #t) #t)))

    (test-case "Custom set-popup-rule! overrides default side and size cleanly"
      (let* ((s0 ((.get ui-ws make-state)))
             (custom ((.get ui-ws make-popup-rule) "*Help*" 'right 0.45 #t 'other 'session #t))
             (s1 ((.get ui-ws add-popup-rule) s0 custom))
             (matched ((.get ui-ws match-popup-rule) s1 "*Help*"))
             (projected ((.get ui-ws project-popup-rules) s1))
             (first-rule (vector-ref projected 0)))
        (check-equal? (.get matched side) 'right)
        (check-equal? (.get matched size) 0.45)
        (check-equal? (.get matched quit) 'other)
        ;; Typed Scheme vector projection preserves custom rule precedence for ABI admission.
        (check-equal? (vector? projected) #t)
        (check-equal? (vector-ref first-rule 0) 'popup-rule)
        (check-equal? (vector-ref first-rule 1) "*Help*")
        (check-equal? (vector-ref first-rule 2) 'right)
        (check-equal? (vector-ref first-rule 3) 0.45)
        (check-equal? (vector-ref first-rule 5) 'other)
        (check-equal? (vector-ref first-rule 6) 'session)))))
