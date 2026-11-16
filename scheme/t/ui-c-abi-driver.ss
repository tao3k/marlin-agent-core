;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Native call-site fixture; this must be compiled before loading.
(import :std/ffi :marlin-agent-core/c-abi/ui)
(export ui-test-open ui-test-project ui-test-dispatch)
(extern ui-test-open ui-test-project ui-test-dispatch)

(begin-foreign
  (namespace ("marlin-agent-core/t/ui-c-abi-driver#" ui-test-open ui-test-project ui-test-dispatch))
  (c-declare "#include <stdint.h>\nextern int64_t marlin_ui_open(int64_t);\nextern int64_t marlin_ui_project_workbench(int64_t);\nextern int64_t marlin_ui_dispatch_event(int64_t);")
  (define ui-test-open (c-lambda (int64) int64 "marlin_ui_open"))
  (define ui-test-project (c-lambda (int64) int64 "marlin_ui_project_workbench"))
  (define ui-test-dispatch (c-lambda (int64) int64 "marlin_ui_dispatch_event")))
