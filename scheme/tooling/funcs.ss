;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later


;;; Pure declarative tool constructors, `deftool` syntax, catalog composition,
;;; and native Scheme-value FFI projection (`scheme/tooling/funcs.ss`).
;;; Zero JSON serialization in Scheme; Rust owns typed admission and provider
;;; JSON Schema synthesis.
(import (only-in :clan/poo/object .get .mix .o seal-object!)
        "./objects.ss")
(export make-tool-param
        make-tool-declaration
        deftool-param
        deftool
        extend-tooling-catalog
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

;;; Concise declarative parameter form used inside `deftool`.
(defrules deftool-param ()
  ((_ (pname ptype pdesc))
   (make-tool-param 'pname type: 'ptype description: pdesc))
  ((_ (pname ptype pdesc required?: req))
   (make-tool-param 'pname type: 'ptype description: pdesc required?: req))
  ((_ (pname ptype pdesc required?: req enum: (ev ...)))
   (make-tool-param 'pname type: 'ptype description: pdesc required?: req enum: '(ev ...)))
  ((_ (pname ptype pdesc enum: (ev ...)))
   (make-tool-param 'pname type: 'ptype description: pdesc enum: '(ev ...))))

;;; Declarative top-level tool definition binding a sealed POO `ToolDeclaration.`.
(defrules deftool ()
  ((_ binding
      name: tname
      description: tdesc
      capability: tcap
      params: (pspec ...))
   (def binding
     (make-tool-declaration
       name: 'tname
       description: tdesc
       capability: 'tcap
       parameters: (list (deftool-param pspec) ...))))
  ((_ binding
      name: tname
      description: tdesc
      capability: tcap
      timeout-seconds: ttimeout
      output-bytes: tlimit
      params: (pspec ...))
   (def binding
     (make-tool-declaration
       name: 'tname
       description: tdesc
       capability: 'tcap
       timeout-seconds: ttimeout
       output-bytes: tlimit
       parameters: (list (deftool-param pspec) ...)))))

;;; Pure functional composition of additional tool declarations onto a catalog.
(def (extend-tooling-catalog base-catalog extra-declarations)
  (seal-object!
    (.mix (.o bind-these: #f
              declarations: (append (.get base-catalog declarations) extra-declarations))
          ToolingCatalog.)))

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
