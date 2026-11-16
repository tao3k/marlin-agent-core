;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :std/test check-equal? test-case test-suite)
        (only-in :clan/poo/object .ref)
        "../brief-to-draft/settings.ss")

(export brief-to-draft-options-test)

(def brief-to-draft-options-test
  (test-suite "Brief-to-Draft Module options"
    (test-case "Project the existing typed options and the imported module"
      (def root (draft-configuration #t 'release-note))
      (check-equal? (agentic-ai-lab-module-names (agentic-ai-lab-module-closure (list root)))
                    '(draft-output brief-input))
      (check-equal? (map (lambda (schema) (.ref schema 'id))
                         (agentic-ai-lab-module-option-schemas root))
                    '("template"))
      (check-equal? (map (lambda (receipt) (.ref receipt 'valid?))
                         (agentic-ai-lab-module-validation-receipts root))
                    '(#t #t)))
    (test-case "Report an invalid option through the existing owner"
      (check-equal? (settings-denial #t 'unknown) '((draft-output "template"))))
    (test-case "Reject the cross-module mismatch before activation"
      (check-equal? (settings-denial #f 'release-note)
                    '((brief-input "require-audience") (draft-output "template")))
      (check-equal? (settings-denial #f 'brief-note) '())
      (check-equal? (settings-denial #t 'release-note) '()))))
