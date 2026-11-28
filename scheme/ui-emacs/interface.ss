;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Declarative Emacs Primitives module: Bookmarks, Kill Ring, Better-Jumper,
;;; and Consult Virtual Sources backed by ASP + Tantivy.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" UiEmacsScope)
        (only-in "./types.ss" UiEmacsScopeContract))
(export UiEmacs)

(.def UiEmacs
  identity: 'ui-emacs
  core: (.o scope-contract: UiEmacsScopeContract
            exports: UiEmacsScope))
