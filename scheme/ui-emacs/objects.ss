;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Declarative POO prototypes for Marlin's Emacs Primitives (`scheme/ui-emacs`):
;;; Bookmarks, Kill Ring, Better-Jumper Stack, and Consult/Imenu Virtual Sources
;;; backed by ASP's native resident Trigram + Tantivy BM25 + AST Symbol engine.
(import (only-in :clan/poo/object .def .o))
(export BookmarkEntry.
        KillRing.
        JumpEntry.
        JumpList.
        ConsultSource.
        DefaultConsultSources
        DefaultEmacsSession)

(.def BookmarkEntry.
  name: ""
  path: ""
  line: 1
  context: ""
  project-root: "")

(.def KillRing.
  entries: '()
  max-size: 60
  yank-index: 0)

(.def JumpEntry.
  buffer-id: "*Preview*"
  path: ""
  line: 1
  label: "")

(.def JumpList.
  entries: '()
  cursor: -1
  max-size: 100)

(.def ConsultSource.
  id: 'buffer
  label: "Buffers"
  narrow-key: "b"
  engine: 'scheme-frame
  priority: 100
  async?: #f)

;; Authoritative Doom Emacs Consult virtual sources & narrowing keys.
(def DefaultConsultSources
  (list
    (.o id: 'buffer
        label: "Open Buffers"
        narrow-key: "b"
        engine: 'scheme-frame
        priority: 100
        async?: #f)
    (.o id: 'recentf
        label: "Recent Files (recentf)"
        narrow-key: "f"
        engine: 'rust-fs-bridge
        priority: 90
        async?: #f)
    (.o id: 'bookmark
        label: "Bookmarks (C-x r b)"
        narrow-key: "m"
        engine: 'scheme-emacs
        priority: 85
        async?: #f)
    (.o id: 'project-file
        label: "Project Files (git ls-files + Orderless)"
        narrow-key: "p"
        engine: 'rust-fs-bridge
        priority: 80
        async?: #t)
    (.o id: 'ripgrep
        label: "Project Search (ASP Trigram + Tantivy BM25)"
        narrow-key: "/"
        engine: 'asp-tantivy
        priority: 75
        async?: #t)
    (.o id: 'imenu
        label: "Symbols & Headings (ASP AST Imenu)"
        narrow-key: "i"
        engine: 'asp-symbol-index
        priority: 70
        async?: #t)
    (.o id: 'line
        label: "Buffer Lines (consult-line)"
        narrow-key: "l"
        engine: 'scheme-frame
        priority: 65
        async?: #f)))

(def DefaultEmacsSession
  (.o bookmarks: '()
      kill-ring: KillRing.
      jump-list: JumpList.
      consult-sources: DefaultConsultSources))
