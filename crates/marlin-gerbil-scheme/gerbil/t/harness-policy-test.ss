;;; -*- Gerbil -*-
;;; Boundary: static gxtest bridge for Marlin Gerbil package policy checks.

(import :gerbil/gambit
        (only-in :std/test check test-case test-suite)
        (only-in :std/srfi/1 filter)
        (only-in :gslph/src/policy/gxtest
                 policy-report
                 display-project-policy-report)
        (only-in :gslph/src/types/facade type-finding-severity))

(export marlin-harness-policy-decode-test-files
        marlin-harness-policy-test-files
        marlin-harness-policy-error-findings
        marlin-harness-policy-error-report
        marlin-harness-policy-test-suite
        harness-policy-test)

;; : String
(def +marlin-harness-policy-files-env+
  "MARLIN_GERBIL_POLICY_TEST_FILES")

;; : (-> MaybeString [String])
(def (marlin-harness-policy-decode-test-files value)
  (if (and value (not (string=? value "")))
    (let (datum (read (open-input-string value)))
      (if (list? datum) datum []))
    []))

;; : (-> Unit [String])
(def (marlin-harness-policy-test-files)
  (marlin-harness-policy-decode-test-files
   (getenv +marlin-harness-policy-files-env+ #f)))

;; : (-> [TypeFinding] [TypeFinding])
(def (marlin-harness-policy-error-findings findings)
  (filter (lambda (finding)
            (equal? (type-finding-severity finding) "error"))
          findings))

;; : (-> PolicyReport [TypeFinding] PolicyReport)
(def (marlin-harness-policy-error-report report errors)
  (hash (schemaId (hash-get report 'schemaId))
        (schemaVersion (hash-get report 'schemaVersion))
        (languageId (hash-get report 'languageId))
        (providerId (hash-get report 'providerId))
        (scope (hash-get report 'scope))
        (requestedFiles (hash-get report 'requestedFiles))
        (status "fail")
        (files (hash-get report 'files))
        (definitions (hash-get report 'definitions))
        (agentRepair (hash-get report 'agentRepair))
        (findings errors)))

;; : (-> [String] TestSuite)
(def (marlin-harness-policy-test-suite files)
  (test-suite "marlin gerbil package policy"
    (test-case "package policy has no error findings for test scope"
      (let* ((report (policy-report "." files))
             (findings (hash-get report 'findings))
             (errors (marlin-harness-policy-error-findings findings)))
        (when (not (null? errors))
          (display-project-policy-report
           (marlin-harness-policy-error-report report errors)))
        (check (length errors) => 0)))))

(def harness-policy-test
  (marlin-harness-policy-test-suite (marlin-harness-policy-test-files)))
