;;; -*- Gerbil -*-
;;; Boundary: Test pins Marlin's Gerbil package policy to the upstream harness library.

(import :gerbil/gambit
        :std/source
        :std/test
        (only-in :std/misc/ports read-file-lines)
        (only-in :std/srfi/13 string-contains)
        (only-in :std/sugar ormap)
        :gslph/src/policy/gxtest)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; : (-> (U #f String) Boolean)
(def (non-empty-string? value)
  (and value (not (string=? value ""))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; : (-> String String String)
(def (env-or name fallback)
  (let (value (getenv name #f))
    (if (non-empty-string? value)
      value
      fallback)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; : (-> String String String)
(def (path-join root suffix)
  (let (root-length (string-length root))
    (if (and (> root-length 0)
             (char=? (string-ref root (- root-length 1)) #\/))
      (string-append root suffix)
      (string-append root "/" suffix))))

;;; Boundary: Test discovery follows the workspace layout without hard-coded
;;; user paths.
;; find-marlin-workspace-root
;;   : (-> String String)
;;   | doc m%
;;       Walk upward from a Gerbil package-local path until the Marlin workspace
;;       layout marker is found.
;;
;;       # Examples
;;
;;       ```scheme
;;       (find-marlin-workspace-root "/repo/crates/marlin-gerbil-scheme/gerbil")
;;       ;; => "/repo"
;;       ```
;;     %
(def (find-marlin-workspace-root start)
  (let loop ((candidate (path-normalize (path-expand start (current-directory)))))
    (if (file-exists?
         (path-join candidate "crates/marlin-gerbil-scheme/gerbil/gerbil.pkg"))
      candidate
      (let (parent (path-directory candidate))
        (if (or (not parent)
                (string=? parent candidate))
          candidate
          (loop parent))))))

;;; Boundary: Harness policy checks run from the Marlin workspace root so
;;; package tests and repo-level provider checks use the same policy view.
;; : String
(def default-harness-gerbil-root
  (path-normalize (path-expand ".." (path-directory (this-source-file)))))

;;; Boundary: Harness policy checks run from the Marlin workspace root so
;;; package tests and repo-level provider checks use the same policy view.
;; : String
(def marlin-workspace-root
  (env-or "MARLIN_WORKSPACE_ROOT"
          (find-marlin-workspace-root default-harness-gerbil-root)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; : (-> (U #f String) (U #f String))
(def (configured-gerbil-root value)
  (cond
   ((not (non-empty-string? value)) #f)
   ((file-exists? (path-join value "gerbil.pkg")) value)
   ((file-exists?
     (path-join value "crates/marlin-gerbil-scheme/gerbil/gerbil.pkg"))
    (path-join value "crates/marlin-gerbil-scheme/gerbil"))
   (else #f)))

;;; Boundary: Project policy fixtures stay rooted at the package that owns
;;; gerbil.pkg.
;; : String
(def harness-gerbil-root
  (or (configured-gerbil-root (getenv "MARLIN_GERBIL_PACKAGE_ROOT" #f))
      default-harness-gerbil-root))

;;; Boundary: Project policy declarations are package-owned, not Rust asset
;;; gates.
;; : String
(def harness-gerbil-pkg
  (path-join harness-gerbil-root "gerbil.pkg"))

;;; Boundary: Build target checks keep loadpath layout under harness policy.
;; : String
(def harness-gerbil-build-script
  (path-join harness-gerbil-root "build.ss"))

;;; Boundary: Modular policy config is applied by the Gerbil language harness.
;; : String
(def harness-gerbil-modularity-policy
  (path-join harness-gerbil-root "harness-policy/gerbil.ss"))

;;; Boundary: Performance source owns a marlin package namespace, not a
;;; package-root shadow namespace.
;; : String
(def harness-gerbil-script-performance-source
  (path-join harness-gerbil-root
             "src/marlin/deck-runtime-script-performance.ss"))

;;; Boundary: Receipt CLI source owns a marlin package namespace, not a
;;; package-root shadow namespace.
;; : String
(def harness-gerbil-policy-receipt-gate-cli-source
  (path-join harness-gerbil-root
             "src/marlin/deck-runtime-policy-receipt-gate-cli.ss"))

;;; Boundary: Performance tests import through the same runtime namespace as
;;; package-declared sources.
;; : String
(def harness-gerbil-performance-test
  (path-join harness-gerbil-root "t/deck-runtime-script-performance-test.ss"))

;;; Boundary: Performance gates import through the same runtime namespace as
;;; package-declared sources.
;; : String
(def harness-gerbil-performance-gate
  (path-join harness-gerbil-root "t/deck-runtime-script-performance-gate.ss"))

;;; Boundary: The harness library smoke checks this policy test owner without
;;; turning import-time gxtest into a full workspace policy scan.
;; : (List String)
(def harness-policy-scope-files
  '("t/harness-policy-test.ss"))

;;; Boundary: Text assertions pin declarative package/build layout without
;;; shelling out to grep.
;; : (-> String String Boolean)
(def (line-contains? line text)
  (if (string-contains line text) #t #f))

;;; Boundary: Text assertions pin declarative package/build layout without
;;; shelling out to grep.
;; file-contains?
;;   : (-> String String Boolean)
;;   | doc m%
;;       Return whether any line in `path` contains `text`, using Gerbil port
;;       utilities instead of shelling out to grep.
;;
;;       # Examples
;;
;;       ```scheme
;;       (file-contains? "gerbil.pkg" "source-scope")
;;       ;; => #t
;;       ```
;;     %
(def (file-contains? path text)
  (ormap (lambda (line) (line-contains? line text))
         (read-file-lines path)))

;;; Boundary: Text assertions pin retired paths without shelling out to grep.
;; : (-> String String Boolean)
(def (file-lacks? path text)
  (not (file-contains? path text)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; : (-> Unit)
(def (check-harness-policy-paths)
  (check (file-exists? marlin-workspace-root) => #t)
  (check (file-exists? harness-gerbil-root) => #t)
  (check (file-exists? harness-gerbil-pkg) => #t)
  (check (file-exists? harness-gerbil-build-script) => #t)
  (check (file-exists? harness-gerbil-modularity-policy) => #t)
  (check (file-exists? harness-gerbil-script-performance-source) => #t)
  (check (file-exists? harness-gerbil-policy-receipt-gate-cli-source) => #t)
  (check (file-exists? harness-gerbil-performance-test) => #t)
  (check (file-exists? harness-gerbil-performance-gate) => #t))

;;; Boundary: Gerbil language harness owns package policy, not Rust asset gates.
;; : (-> Unit)
(def (check-harness-policy-declares-module-layout)
  (check (file-contains? harness-gerbil-pkg "source-scope") => #t)
  (check (file-contains? harness-gerbil-pkg "runtime-roots: (\"src\")") => #t)
  (check (file-contains? harness-gerbil-pkg "modularity-policy") => #t)
  (check (file-contains? harness-gerbil-pkg "config: \"harness-policy/gerbil.ss\"")
         => #t)
  (check (file-contains? harness-gerbil-modularity-policy "(modularity-policy")
         => #t)
  (check (file-contains? harness-gerbil-build-script ":clan/building") => #t)
  (check (file-contains? harness-gerbil-build-script "all-gerbil-modules") => #t)
  (check (file-contains? harness-gerbil-build-script
                         "+marlin-special-source-files+")
         => #t)
  (check (file-contains? harness-gerbil-build-script
                         "+marlin-build-phase-receipt-schema+")
         => #t)
  (check (file-contains? harness-gerbil-build-script
                         "marlin-build-emit-phase-receipt")
         => #t)
  (check (file-contains? harness-gerbil-build-script
                         "marlin-build-cache-fresh?")
         => #t)
  (check (file-contains? harness-gerbil-build-script "phase-skip") => #t)
  (check (file-contains? harness-gerbil-build-script "cacheStamp") => #t)
  (check (file-contains? harness-gerbil-build-script "\"GERBIL_BUILD_CORES\"")
         => #t)
  (check (file-contains? harness-gerbil-build-script "##cpu-count") => #t)
  (check (file-lacks? harness-gerbil-build-script "GSLPH_TEST_JOBS") => #t)
  (check (file-lacks? harness-gerbil-build-script "(* 2") => #t)
  (check (file-lacks? harness-gerbil-build-script "stage-native-aot") => #t)
  (check (file-lacks? harness-gerbil-build-script "check-main") => #t)
  (check (file-lacks? harness-gerbil-build-script
                      "+marlin-native-aot-only-modules+")
         => #t)
  (check (file-lacks? harness-gerbil-build-script "marlin-package-test") => #t)
  (check (file-lacks? harness-gerbil-build-script "\"src/config-interface/modules/lib\"")
         => #t)
  (check (file-lacks? harness-gerbil-build-script "\"modules/config-interface/modules/lib\"")
         => #t))

;;; Boundary: Source and tests share one marlin module namespace.
;; : (-> Unit)
(def (check-harness-module-namespace)
  (check (file-contains? harness-gerbil-script-performance-source
                         "package: marlin")
         => #t)
  (check (file-contains? harness-gerbil-policy-receipt-gate-cli-source
                         "package: marlin")
         => #t)
  (check (file-contains? harness-gerbil-performance-test
                         ":marlin/deck-runtime-script-performance")
         => #t)
  (check (file-contains? harness-gerbil-performance-gate
                         ":marlin/deck-runtime-script-performance")
         => #t)
  (check (file-lacks? harness-gerbil-performance-test
                      ":marlin-deck-runtime/src")
         => #t)
  (check (file-lacks? harness-gerbil-performance-gate
                      ":marlin-deck-runtime/src")
         => #t))

;;; Boundary: Policy execution goes through the Gerbil harness library API.
;; : (-> Unit)
(def (check-harness-policy)
  (let (report (gslph/src/policy/gxtest#policy-report
                harness-gerbil-root
                harness-policy-scope-files))
    (when (not (equal? (hash-get report 'status) "pass"))
      (gslph/src/policy/gxtest#display-project-policy-report report))
    (check (hash-get report 'scope) => "files")
    (check (> (hash-get report 'files) 0) => #t)
    (check (hash-get report 'status) => "pass")
    (check (hash-get report 'findings) => [])))

(check-harness-policy-paths)
(check-harness-policy-declares-module-layout)
(check-harness-module-namespace)
(check-harness-policy)
