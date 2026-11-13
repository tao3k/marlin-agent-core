;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; A contributed component and method bundle selected from the real UI scope.
(import (only-in :clan/poo/object .get .o)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports)
        (only-in :tao3k/yuanxu/poo-clos/generic-syntax .defgeneric .defmethod)
        (only-in :tao3k/yuanxu/poo-clos/method-bundle-syntax .defmethod-bundle))
(export test-extension test-methods test-forged-methods)

(def ui (.get marlin-agent-core.init.exports Marlin ui))
(def test-extension
  (.o (:: @ (.get ui components extension)) identity: 'test/extension
      kind: 'extension label: "Test extension" focusable: #t
      content: (list
                 (.o (:: @ (.get ui components text))
                     kind: 'text text: "Scheme component body")
                 (.o (:: @ (.get ui components field))
                     kind: 'field label: "Source" value: "selected module"))))
(def submit-event (.get ui events submit))
(.defgeneric test-event (component event) (protocol (.get ui event-protocol)))
(.defmethod-bundle test-methods (.get ui event-protocol)
  (.defmethod (test-event (component (prototype test-extension))
                          (event (prototype submit-event)))
    (.o kind: 'focus identity: 'composer)))
(.defmethod-bundle test-forged-methods (.get ui event-protocol)
  (.defmethod (test-event (component (prototype test-extension))
                          (event (prototype submit-event)))
    (.o kind: 'submit-draft)))
