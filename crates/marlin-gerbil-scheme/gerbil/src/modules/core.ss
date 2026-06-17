;;; -*- Gerbil -*-
;;; Boundary: Core user module API and import/config helpers.

package: modules

(import (only-in :clan/poo/object .all-slots .get .has? .o .ref object?)
        (only-in :clan/poo/type String)
        :modules/kinds)

(export marlin-module-interface
        marlin-string-required
        marlin-string-constant
        marlin-string-default
        marlin-string-optional
        marlin-module-object-has-slot?
        marlin-module-object-ref/default
        marlinModules
        marlin-import
        marlin-imports
        marlin-imports-append
        marlin-extensions
        marlin-extensions-append
        marlin-import?
        marlin-module-config?
        marlin-module-import-source-ref?
        marlin-module-import-local-source?
        marlin-module-import-normalize-source
        marlin-source-ref
        marlin-local-source)

;;; Boundary: Interface objects carry schemas outside user config records.
;; MarlinResult <- MarlinInput
(def (marlin-module-interface interface-id-value schema-object metadata-value)
  (.o (:: @ (list marlin-module-prototype))
      id: interface-id-value
      schemas: schema-object
      metadata: metadata-value))

;;; Boundary: String option helpers keep user interface modules concise.
;; MarlinResult <- MarlinInput
(def (marlin-string-required)
  (.o type: String))

;;; Boundary: String constant helpers model fixed interface values.
;; MarlinResult <- MarlinInput
(def (marlin-string-constant constant-value)
  (.o type: String
      constant: constant-value))

;;; Boundary: String default helpers model optional defaults.
;; MarlinResult <- MarlinInput
(def (marlin-string-default default-value)
  (.o type: String
      default: default-value))

;;; Boundary: String optional helpers model optional schema slots.
;; MarlinResult <- MarlinInput
(def (marlin-string-optional)
  (.o type: String
      optional?: #t))

;;; Boundary: Config object lookup supports a record-like user interface.
;; MarlinResult <- MarlinInput
(def (marlin-module-object-has-slot? object slot-name)
  (member slot-name (.all-slots object)))

;;; Boundary: Missing config fields fall back to the module interface defaults.
;; MarlinResult <- MarlinInput
(def (marlin-module-object-ref/default object slot-name default-value)
  (if (marlin-module-object-has-slot? object slot-name)
    (.ref object slot-name)
    default-value))

;;; Boundary: Public user API mirrors typed config records from module systems.
;; MarlinResult <- MarlinInput
(def (marlinModules interface module-config)
  (let ((config-values
         (marlin-module-object-ref/default
          module-config
          'config
          (.o))))
    (.o (:: @ (list interface))
        id:
        (marlin-module-object-ref/default module-config 'id (.get interface id))
        imports:
        (marlin-module-object-ref/default module-config 'imports '())
        extensions:
        (marlin-module-object-ref/default module-config 'extensions '())
        scripts:
        (marlin-module-object-ref/default module-config 'scripts '())
        options: config-values
        metadata:
        (marlin-module-object-ref/default
         module-config
         'metadata
         (.get interface metadata)))))

;;; Boundary: Local source objects mirror Jsonnet-style structured config.
;; MarlinResult <- MarlinInput
(def (marlin-local-source source-path)
  (.o kind: marlin-import-local-source-kind
      path: source-path))

;;; Boundary: Source refs wrap concrete source kinds for future package sources.
;; MarlinResult <- MarlinInput
(def (marlin-source-ref source-value)
  (.o kind: marlin-import-source-ref-kind
      source: source-value))

;;; Boundary: Source-ref detection keeps normalization typed by object kind.
;; MarlinResult <- MarlinInput
(def (marlin-module-import-source-ref? value)
  (and (object? value)
       (.has? value kind)
       (string=? (.get value kind) marlin-import-source-ref-kind)))

;;; Boundary: Local-source detection keeps normalization typed by object kind.
;; MarlinResult <- MarlinInput
(def (marlin-module-import-local-source? value)
  (and (object? value)
       (.has? value kind)
       (string=? (.get value kind) marlin-import-local-source-kind)))

;;; Boundary: User imports accept path strings or explicit source objects.
;; MarlinResult <- MarlinInput
(def (marlin-module-import-normalize-source source-value)
  (cond
   ((string? source-value)
    (marlin-source-ref (marlin-local-source source-value)))
   ((marlin-module-import-source-ref? source-value)
    source-value)
   ((marlin-module-import-local-source? source-value)
    (marlin-source-ref source-value))
   (else source-value)))

;;; Boundary: Import specs keep user config close to POO extension objects.
;; MarlinResult <- MarlinInput
(def (make-marlin-import source-ref-value profile-value)
  (.o kind: marlin-module-import-kind
      source-ref: source-ref-value
      profile: profile-value))

;;; Boundary: Public import helper accepts profile or path/profile forms.
;; MarlinResult <- MarlinInput
(def (marlin-import . import-values)
  (cond
   ((= (length import-values) 1)
    (make-marlin-import #f (car import-values)))
   ((= (length import-values) 2)
    (make-marlin-import
     (marlin-module-import-normalize-source (car import-values))
     (cadr import-values)))
    (else
    (error "marlin-import expects profile or source/profile"))))

;;; Boundary: Import lists are explicit values consumed by POO slot methods.
;; MarlinResult <- MarlinInput
(def (marlin-imports . import-values)
  import-values)

;;; Boundary: POO slot methods lazily append child imports to inherited imports.
;; MarlinResult <- MarlinInput
(def (marlin-imports-append inherited-imports direct-imports)
  (append inherited-imports direct-imports))

;;; Boundary: Extension lists let agent-authored POO objects stay first-class.
;; MarlinResult <- MarlinInput
(def (marlin-extensions . extension-values)
  extension-values)

;;; Boundary: POO slot methods lazily append child extension objects.
;; MarlinResult <- MarlinInput
(def (marlin-extensions-append inherited-extensions direct-extensions)
  (append inherited-extensions direct-extensions))

;;; Boundary: Import spec detection is typed by kind, not by list shape.
;; MarlinResult <- MarlinInput
(def (marlin-import? value)
  (and (object? value)
       (.has? value kind)
       (string=? (.get value kind) marlin-module-import-kind)))

;;; Boundary: Module config detection is typed by kind, not by source shape.
;; MarlinResult <- MarlinInput
(def (marlin-module-config? value)
  (and (object? value)
       (.has? value kind)
       (or (string=? (.get value kind) marlin-modules-kind)
           (string=? (.get value kind) marlin-policy-module-kind))))
