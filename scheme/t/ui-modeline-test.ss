;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import :std/test
        (only-in :clan/poo/object .get)
        :marlin-agent-core/init)
(export ui-modeline-test)

(def ui-modeline (.get marlin-agent-core.init.exports Marlin ui-modeline))

(def ui-modeline-test
  (test-suite "modern doom modeline presets, responsive visibility & shrink-path"
    (test-case "canonical Doom presets declare ordered LHS and RHS segments"
      (let* ((main ((.get ui-modeline preset-spec) 'main))
             (dash ((.get ui-modeline preset-spec) 'dashboard))
             (helm ((.get ui-modeline preset-spec) 'helm)))
        (check-equal? (.get main lhs)
          '(bar window-state window-number modals buffer-info remote-host position))
        (check-equal? (.get main rhs)
          '(status-items deep-think workspace gates model vcs major-mode))
        (check-equal? (and (memq 'workspace-path (.get dash lhs)) #t) #t)
        (check-equal? (.get helm lhs) '(bar modals helm-title helm-count))))

    (test-case "width-aware responsive visibility + single vs multi-window + inactive pane dimming"
      (let* ((wide-single ((.get ui-modeline visible-segments) 'main 120 #f #t))
             (compact-multi ((.get ui-modeline visible-segments) 'main 48 #t #t))
             (inactive-multi ((.get ui-modeline visible-segments) 'main 64 #t #f)))
        ;; Single window hides window-number; keeps wide segments like position & vcs.
        (check-equal? (and (memq 'window-number (.get wide-single lhs)) #t) #f)
        (check-equal? (and (memq 'position (.get wide-single lhs)) #t) #t)
        (check-equal? (and (memq 'vcs (.get wide-single rhs)) #t) #t)
        ;; Compact multi-window shows window-number & essential segments while pruning wide-only items.
        (check-equal? (and (memq 'window-number (.get compact-multi lhs)) #t) #t)
        (check-equal? (and (memq 'position (.get compact-multi lhs)) #t) #f)
        (check-equal? (and (memq 'vcs (.get compact-multi rhs)) #t) #f)
        (check-equal? (and (memq 'model (.get compact-multi rhs)) #t) #t)
        ;; Inactive pane shows only focused-pane-dimmed subset (bar, window-state, window-number, buffer-info, major-mode).
        (check-equal? (.get inactive-multi lhs) '(bar window-state window-number buffer-info))
        (check-equal? (.get inactive-multi rhs) '(major-mode))))

    (test-case "Doom shrink-path compresses intermediate directories cleanly"
      (check-equal? ((.get ui-modeline shrink-path) "~/ghq/github.com/tao3k/marlin-agent-core/")
                    "~/g/g/t/marlin-agent-core/")
      (check-equal? ((.get ui-modeline shrink-path) "/Users/guangtao/.config/marlin/ui")
                    "/U/g/.c/m/ui")
      (check-equal? ((.get ui-modeline shrink-path) "*Dashboard*")
                    "*Dashboard*"))))
