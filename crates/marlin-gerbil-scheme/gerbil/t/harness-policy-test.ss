;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.

(import :std/test)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (env-or name fallback)
  (or (getenv name #f) fallback))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (home-path suffix)
  (let (home (getenv "HOME" #f))
    (if home
      (string-append home suffix)
      suffix)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def default-harness-root
  (home-path "/ghq/github.com/tao3k/agent-semantic-protocols/languages/gerbil-scheme-language-project-harness"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (shell-quote value)
  (string-append
   "'"
   (apply
    string-append
    (map
     (lambda (ch)
       (if (char=? ch #\')
         "'\\''"
         (string ch)))
     (string->list value)))
   "'"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def harness-root
  (env-or "MARLIN_GERBIL_SCHEME_HARNESS_ROOT" default-harness-root))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def harness-src
  (string-append harness-root "/src"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def harness-main
  (string-append harness-root "/bin/gerbil-scheme-harness.ss"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def default-marlin-workspace-root
  (string-append (current-directory) "/../../.."))

;;; Boundary: Harness policy checks run from the Marlin workspace root so
;;; local package tests and repo-level provider checks use the same policy view.
;; MarlinResult <- MarlinInput
(def harness-package-root
  (env-or "MARLIN_GERBIL_PACKAGE_ROOT" default-marlin-workspace-root))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (path-join root suffix)
  (let (root-length (string-length root))
    (if (and (> root-length 0)
             (char=? (string-ref root (- root-length 1)) #\/))
      (string-append root suffix)
      (string-append root "/" suffix))))

;;; Boundary: Project policy fixtures stay rooted at the workspace package.
;; MarlinResult <- MarlinInput
(def harness-gerbil-pkg
  (path-join harness-package-root "crates/marlin-gerbil-scheme/gerbil/gerbil.pkg"))

;;; Boundary: Build target checks keep loadpath layout under harness policy.
;; MarlinResult <- MarlinInput
(def harness-gerbil-build-script
  (path-join harness-package-root "crates/marlin-gerbil-scheme/gerbil/build.ss"))

;;; Boundary: Modular policy config is applied by the Gerbil language harness.
;; MarlinResult <- MarlinInput
(def harness-gerbil-modularity-policy
  (path-join harness-package-root
             "crates/marlin-gerbil-scheme/gerbil/harness-policy/gerbil.ss"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def harness-loadpath
  (let (current-loadpath (getenv "GERBIL_LOADPATH" #f))
    (if current-loadpath
      (string-append current-loadpath ":" harness-src)
      harness-src)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def gxi-bin
  (env-or "GERBIL" "/usr/local/bin/gxi"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def harness-home
  (env-or "HOME" ""))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def harness-path
  (env-or "PATH" "/usr/local/bin:/usr/bin:/bin"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (harness-check-command-for root)
  (string-append
   "cd " (shell-quote root)
   " && env -i"
   " HOME=" (shell-quote harness-home)
   " PATH=" (shell-quote harness-path)
   " GERBIL_LOADPATH=" (shell-quote harness-loadpath)
   " " (shell-quote gxi-bin)
   " " (shell-quote harness-main)
   " check ."))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (harness-check-command)
  (harness-check-command-for harness-package-root))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def policy-negative-root
  (path-join harness-package-root ".run/marlin-harness-policy-negative"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def policy-negative-output
  (path-join harness-package-root ".run/marlin-harness-policy-negative.out"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (string-prefix? prefix value)
  (let ((prefix-length (string-length prefix))
        (value-length (string-length value)))
    (and (<= prefix-length value-length)
         (andmap
          (lambda (index)
            (char=? (string-ref prefix index)
                    (string-ref value index)))
          (list-tabulate prefix-length identity)))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (policy-negative-path? path)
  (or (string=? path policy-negative-root)
      (string-prefix?
       (string-append policy-negative-root "/")
       path)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (ensure-dir path)
  (with-catch
   (lambda (_) #f)
   (lambda () (create-directory path))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (write-text path text)
  (delete-file-if-exists path)
  (call-with-output-file path
    (lambda (port) (display text port))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (delete-file-if-exists path)
  (with-catch
   (lambda (_) #f)
   (lambda () (delete-file path))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (delete-policy-fixture-tree path)
  (when (and (policy-negative-path? path)
             (file-exists? path))
    (if (eq? (file-type path) 'directory)
      (begin
        (for-each
         (lambda (entry)
           (delete-policy-fixture-tree (path-join path entry)))
         (directory-files path))
        (delete-directory path))
      (delete-file path))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (reset-negative-policy-fixture)
  (delete-policy-fixture-tree policy-negative-root)
  (delete-file-if-exists policy-negative-output))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (write-negative-policy-fixture)
  (let ((owner (string-append policy-negative-root "/src/macros")))
    (reset-negative-policy-fixture)
    (ensure-dir ".run")
    (ensure-dir policy-negative-root)
    (ensure-dir (string-append policy-negative-root "/src"))
    (ensure-dir owner)
    (write-text
     (string-append policy-negative-root "/gerbil.pkg")
     ";;; -*- Gerbil -*-\n(package: sample/macros\n  policy: ((agent-policy enabled-rules: (\"GERBIL-SCHEME-AGENT-R011\"))))\n")
    (write-text
     (string-append owner "/core.ss")
     ";;; -*- Gerbil -*-\n(package: sample/macros)\n(defsyntax with-order\n  (lambda (stx) stx))\n")))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (negative-policy-check-command)
  (string-append
   (harness-check-command-for policy-negative-root)
   " > " (shell-quote policy-negative-output)
   " 2>&1; status=$?; cat "
   (shell-quote policy-negative-output)
   "; exit $status"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (policy-output-contains-rule-command rule-id)
  (string-append
   "grep -q "
   (shell-quote rule-id)
   " "
   (shell-quote policy-negative-output)))

;;; Boundary: Text assertions pin gerbil.pkg policy and build layout.
;; MarlinResult <- MarlinInput
(def (file-contains-command path text)
  (string-append
   "grep -F -q "
   (shell-quote text)
   " "
   (shell-quote path)))

;;; Boundary: Text assertions pin gerbil.pkg policy and build layout.
;; MarlinResult <- MarlinInput
(def (file-lacks-command path text)
  (string-append
   "if grep -F -q "
   (shell-quote text)
   " "
   (shell-quote path)
   "; then exit 1; else exit 0; fi"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-harness-policy-paths)
  (check (file-exists? harness-package-root) => #t)
  (check (file-exists? harness-src) => #t)
  (check (file-exists? harness-main) => #t)
  (check (file-exists? harness-gerbil-pkg) => #t)
  (check (file-exists? harness-gerbil-build-script) => #t)
  (check (file-exists? harness-gerbil-modularity-policy) => #t))

;;; Boundary: Gerbil language harness owns package policy, not Rust asset gates.
;; MarlinResult <- MarlinInput
(def (check-harness-policy-declares-module-layout)
  (check (shell-command
          (file-contains-command harness-gerbil-pkg "source-scope"))
         => 0)
  (check (shell-command
          (file-contains-command harness-gerbil-pkg "runtime-roots: (\"src\")"))
         => 0)
  (check (shell-command
          (file-contains-command
           harness-gerbil-pkg
           "modularity-policy"))
         => 0)
  (check (shell-command
          (file-contains-command
           harness-gerbil-pkg
           "config: \"harness-policy/gerbil.ss\""))
         => 0)
  (check (shell-command
          (file-contains-command
           harness-gerbil-modularity-policy
           "(modularity-policy"))
         => 0)
  (check (shell-command
          (file-contains-command harness-gerbil-build-script "\"src/modules/lib\""))
         => 0)
  (check (shell-command
          (file-lacks-command harness-gerbil-build-script "\"modules/marlin/modules/lib\""))
         => 0))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-harness-policy)
  (check (shell-command (harness-check-command)) => 0))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-harness-policy-applies-rules)
  (write-negative-policy-fixture)
  (shell-command (negative-policy-check-command))
  (check (shell-command
          (policy-output-contains-rule-command "status=fail"))
         => 0)
  (check (shell-command
          (policy-output-contains-rule-command "GERBIL-SCHEME-AGENT-R011"))
         => 0)
  (reset-negative-policy-fixture))

(check-harness-policy-paths)
(check-harness-policy-declares-module-layout)
(check-harness-policy)
(check-harness-policy-applies-rules)
