;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/mop Bool)
        (only-in :clan/poo/object .o .ref)
        (only-in :clan/poo/type Enum Record)
        (only-in :agentic-ai-lab/modules/module-authoring/module-interface
                 yuanxu-module-interface)
        (only-in :agentic-ai-lab/modules/module-authoring/module-descriptor
                 agentic-ai-lab-modules agentic-ai-lab-module-closure
                 agentic-ai-lab-module-names)
        (only-in :agentic-ai-lab/modules/module-authoring/module-option-projection
                 agentic-ai-lab-module-option-schemas
                 agentic-ai-lab-module-validation-receipts)
        (only-in :tao3k/yuanxu/module-system/source/imports yuanxu-import))

(export draft-configuration settings-denial
        agentic-ai-lab-module-closure agentic-ai-lab-module-names
        agentic-ai-lab-module-option-schemas agentic-ai-lab-module-validation-receipts)

(def BriefInput
  (yuanxu-module-interface "BriefInput"
    (Record require-audience: [Bool default: #t]) '()))

(def DraftOutput
  (yuanxu-module-interface "DraftOutput"
    (Record template: [(Enum brief-note release-note) default: 'brief-note]) '()))

(def (draft-configuration audience-required? template-value)
  (def brief
    (agentic-ai-lab-modules BriefInput
      (.o id: 'brief-input config: (.o require-audience: audience-required?))))
  (agentic-ai-lab-modules DraftOutput
    (.o id: 'draft-output imports: (list (yuanxu-import ":brief-to-draft#brief-input" brief))
        config: (.o template: template-value))))

;; The fixed workflow requires the producer to guarantee the field consumed
;; by the release-note template. Lab checks each Module's own Record first.
(def (settings-denial audience-required? template-value)
  (let* ((root (draft-configuration audience-required? template-value))
         (receipts (agentic-ai-lab-module-validation-receipts root))
         (invalid (filter (lambda (receipt) (not (.ref receipt 'valid?))) receipts)))
    (cond
      ((pair? invalid)
       (map (lambda (receipt) (list (.ref receipt 'source-module) (.ref receipt 'id))) invalid))
      ((and (eq? template-value 'release-note) (not audience-required?))
       '((brief-input "require-audience") (draft-output "template")))
      (else '()))))
