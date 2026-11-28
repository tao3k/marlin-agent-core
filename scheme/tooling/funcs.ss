;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later


;;; Pure declarative tool constructors and native Scheme-value FFI projection
;;; (`scheme/tooling/funcs.ss`). Zero JSON serialization in Scheme; Rust owns
;;; typed admission and provider JSON Schema synthesis.
(import (only-in :clan/poo/object .get .mix .o seal-object!)
        "./objects.ss")
(export make-tool-param
        make-tool-declaration
        project-tool-param
        project-tool
        project-tooling)

(def (make-tool-param param-name
                      type: (param-type 'string)
                      description: (param-desc "")
                      required?: (param-required? #t)
                      enum: (param-enum #f))
  (seal-object!
    (.mix (.o bind-these: #f
              name: param-name
              type: param-type
              description: param-desc
              required?: param-required?
              enum: param-enum)
          ToolParam.)))

(def (make-tool-declaration name: tool-name
                            description: tool-desc
                            capability: tool-cap
                            parameters: tool-params
                            governance: (tool-gov 'ask)
                            timeout-seconds: (tool-timeout 30)
                            output-bytes: (tool-output-bytes 65536))
  (seal-object!
    (.mix (.o bind-these: #f
              name: tool-name
              description: tool-desc
              capability: tool-cap
              governance: tool-gov
              parameters: tool-params
              timeout-seconds: tool-timeout
              output-bytes: tool-output-bytes)
          ToolDeclaration.)))

(def (project-tool-param param)
  (let ((allowed (.get param enum)))
    (vector (.get param name)
            (.get param type)
            (.get param description)
            (.get param required?)
            (if (list? allowed) (list->vector allowed) #f))))

(def (project-tool declaration)
  (vector (.get declaration name)
          (.get declaration description)
          (.get declaration capability)
          (.get declaration governance)
          (list->vector (map project-tool-param (.get declaration parameters)))
          (.get declaration timeout-seconds)
          (.get declaration output-bytes)))

(def (project-tooling catalog)
  (list->vector (map project-tool (.get catalog declarations))))
