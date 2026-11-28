;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure functional projection of declarative POO Agent Testing Suites.
(import (only-in :clan/poo/object .get)
        "./objects.ss"
        "./test-case-one.ss"
        "./test-case-two.ss"
        "./trajectory.ss")
(export project-testing-governance
        project-testing-scenario
        resolve-testing-suite
        project-testing-suite
        verify-testing-trajectory)

(def (project-cedar-entry entry)
  (vector (car entry) (cadr entry)))

(def (project-testing-governance gov)
  (vector (.get gov id)
          (.get gov write-allowed?)
          (list->vector (.get gov allowed-capabilities))
          (list->vector (.get gov denied-capabilities))
          (list->vector (.get gov allowed-tools))
          (list->vector (.get gov denied-tools))
          (list->vector (map project-cedar-entry (.get gov cedar-policies)))
          (.get gov denial-message)))

(def (project-checkpoint cp)
  (vector (car cp) (list->vector (cadr cp))))

(def (project-testing-scenario scenario)
  (vector (.get scenario id)
          (.get scenario title)
          (.get scenario message)
          (.get scenario target-max-rounds)
          (.get scenario max-rounds)
          (.get scenario max-tool-calls)
          (.get scenario require-workspace-search?)
          (project-testing-governance (.get scenario governance))
          (list->vector (map project-checkpoint (.get scenario checkpoints)))
          (.get scenario enable-reflection?)
          (list->vector (.get scenario required-tiers))))

(def (resolve-testing-suite selector)
  (cond
    ((or (eq? selector 'complex)
         (eq? selector 'case-two)
         (and (string? selector)
              (or (string=? selector "complex")
                  (string=? selector "case-two"))))
     ComplexCodingTestingSuiteValue)
    ((or (void? selector)
         (not selector)
         (eq? selector 'default)
         (eq? selector 'case-one)
         (and (string? selector)
              (or (string=? selector "default")
                  (string=? selector "case-one"))))
     SearchTestingSuiteValue)
    (else selector)))

(def (project-testing-suite (suite-or-selector SearchTestingSuiteValue))
  (let ((suite (resolve-testing-suite suite-or-selector)))
    (vector 'testing-suite
            (.get suite id)
            (list->vector (map project-testing-scenario (.get suite scenarios))))))
