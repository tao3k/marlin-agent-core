;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import :std/test
        (only-in :clan/poo/object .get)
        (only-in "../ui-emacs/config.ss" UiEmacsScope))
(export ui-emacs-test)

(def ui-emacs-test
  (test-suite "Marlin Gerbil Scheme Emacs Primitives (Bookmarks, Kill Ring, Better-Jumper, Consult)"
    (test-case "bookmarks set, overwrite, jump, and delete purely"
      (let* ((s0 ((.get UiEmacsScope make-session)))
             (s1 ((.get UiEmacsScope bookmark-set) s0 "magit-bridge" "crates/app/magit_bridge.rs" 42 "refresh_snapshot" "/repo"))
             (s2 ((.get UiEmacsScope bookmark-set) s1 "fs-bridge" "crates/app/fs_bridge.rs" 108 "query_fs_records" "/repo"))
             ;; Re-setting existing bookmark replaces cleanly without duplicate.
             (s3 ((.get UiEmacsScope bookmark-set) s2 "magit-bridge" "crates/app/magit_bridge.rs" 99 "updated" "/repo"))
             (bm ((.get UiEmacsScope bookmark-jump) s3 "magit-bridge"))
             (s4 ((.get UiEmacsScope bookmark-delete) s3 "fs-bridge")))
        (check (length ((.get UiEmacsScope bookmark-list) s3)) => 2)
        (check (.get bm line) => 99)
        (check (.get bm context) => "updated")
        (check (length ((.get UiEmacsScope bookmark-list) s4)) => 1)
        (check ((.get UiEmacsScope bookmark-jump) s4 "fs-bridge") => #f)))

    (test-case "kill-ring deduplicates, rotates with yank-pop, and preserves latest on push"
      (let* ((s0 ((.get UiEmacsScope make-session)))
             (s1 ((.get UiEmacsScope kill-push) s0 "first"))
             (s2 ((.get UiEmacsScope kill-push) s1 "second"))
             (s3 ((.get UiEmacsScope kill-push) s2 "third"))
             (y0 ((.get UiEmacsScope yank-current) s3)))
        (check y0 => "third")
        (let-values (((s4 y1) ((.get UiEmacsScope yank-pop) s3)))
          (check y1 => "second")
          (let-values (((s5 y2) ((.get UiEmacsScope yank-pop) s4)))
            (check y2 => "first")
            ;; Pushing an existing item moves it back to front cleanly.
            (let ((s6 ((.get UiEmacsScope kill-push) s5 "first")))
              (check ((.get UiEmacsScope yank-current) s6) => "first"))))))

    (test-case "better-jumper records jumps, steps back/forward, and truncates forward tail on new jump"
      (let* ((s0 ((.get UiEmacsScope make-session)))
             (s1 ((.get UiEmacsScope jump-push) s0 "*Preview*" "a.rs" 10 "fn alpha"))
             (s2 ((.get UiEmacsScope jump-push) s1 "*Preview*" "b.rs" 20 "fn beta"))
             (s3 ((.get UiEmacsScope jump-push) s2 "*Preview*" "c.rs" 30 "fn gamma")))
        (let-values (((s4 j-back) ((.get UiEmacsScope jump-back) s3)))
          (check (.get j-back path) => "b.rs")
          (check (.get j-back line) => 20)
          ;; New jump from mid-stack replaces forward tail (c.rs) with d.rs.
          (let ((s5 ((.get UiEmacsScope jump-push) s4 "*Preview*" "d.rs" 40 "fn delta")))
            (let-values (((s6 j-fwd) ((.get UiEmacsScope jump-forward) s5)))
              (check j-fwd => #f))
            (let-values (((s7 j-prev) ((.get UiEmacsScope jump-back) s5)))
              (check (.get j-prev path) => "b.rs")
              (let-values (((s8 j-again) ((.get UiEmacsScope jump-forward) s7)))
                (check (.get j-again path) => "d.rs")))))))

    (test-case "native FFI projection emits Consult sources backed by ASP + Tantivy"
      (let ((proj ((.get UiEmacsScope project-consult-sources) #f)))
        (check (>= (vector-length proj) 7) => #t)
        (check (vector-ref (vector-ref proj 4) 0) => "ripgrep")
        (check (vector-ref (vector-ref proj 4) 3) => "asp-tantivy")
        (check (vector-ref (vector-ref proj 5) 0) => "imenu")
        (check (vector-ref (vector-ref proj 5) 3) => "asp-symbol-index")))

    (test-case "global mark ring pushes, deduplicates, and rotates cleanly"
      (let* ((s0 ((.get UiEmacsScope make-session)))
             (s1 ((.get UiEmacsScope push-mark) s0 "*Preview*" "a.rs" 12))
             (s2 ((.get UiEmacsScope push-mark) s1 "*Preview*" "b.rs" 44)))
        (let-values (((s3 m1) ((.get UiEmacsScope pop-global-mark) s2)))
          (check (.get m1 path) => "b.rs")
          (check (.get m1 line) => 44)
          (let-values (((s4 m2) ((.get UiEmacsScope pop-global-mark) s3)))
            (check (.get m2 path) => "a.rs")
            (check (.get m2 line) => 12)))))

    (test-case "depth-ordered Emacs hook pipeline respects negative/zero/positive depth"
      (let* ((s0 ((.get UiEmacsScope make-session)))
             (s1 ((.get UiEmacsScope add-hook) s0 'after-save-hook 'run-linter 10))
             (s2 ((.get UiEmacsScope add-hook) s1 'after-save-hook 'format-buffer -50))
             (s3 ((.get UiEmacsScope add-hook) s2 'after-save-hook 'update-asp-index 0))
             (s4 ((.get UiEmacsScope remove-hook) s3 'after-save-hook 'run-linter)))
        (check ((.get UiEmacsScope run-hooks) s3 'after-save-hook)
               => '(format-buffer update-asp-index run-linter))
        (check ((.get UiEmacsScope run-hooks) s4 'after-save-hook)
               => '(format-buffer update-asp-index))))))
