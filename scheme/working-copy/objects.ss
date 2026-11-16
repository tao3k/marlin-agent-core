;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .o))

(export WorkingCopyOperations)

(def (choose-working-copy-provider request)
  (unless (and (vector? request) (= 4 (vector-length request)))
    (error "invalid working-copy policy input"))
  (let ((operation (vector-ref request 0))
        (git? (vector-ref request 1))
        (worktrunk? (vector-ref request 2))
        (prefer-worktrunk? (vector-ref request 3)))
    (unless (and (boolean? git?) (boolean? worktrunk?) (boolean? prefer-worktrunk?))
      (error "working-copy provider options must be booleans"))
    (case operation
      ((pull-request-checkout) (and worktrunk? 'worktrunk))
      ((create switch list remove)
       (cond ((and worktrunk? prefer-worktrunk?) 'worktrunk)
             (git? 'git-worktree)
             (worktrunk? 'worktrunk)
             (else #f)))
      (else (error "unknown working-copy operation" operation)))))

(def WorkingCopyOperations (.o select-provider: choose-working-copy-provider))
