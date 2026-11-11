;;; Read-only snapshots of Gerbil's existing compiler ownership graph.
;;; No Module lookup, promise force, source admission or retained compiler mutation.
package: marlin-benchmark
(import (only-in :gerbil/expander
  __module-registry expander-context? expander-context-table
  phi-context? phi-context-super phi-context-up phi-context-down
  module-context? module-context-code module-context-e
  module-context-import module-context-export
  prelude-context? prelude-context-e prelude-context-import
  module-import? module-import-source module-export? module-export-context
  import-set? import-set-source import-set-imports
  export-set? export-set-source export-set-exports))
(export compiler-roots-snapshot)

(def (compiler-linkage-topology)
  (def seen (make-hash-table-eq))
  (def total 0) (def modules 0) (def preludes 0) (def bindings 0)
  (def bodies 0) (def pending 0) (def ready 0) (def imports 0) (def exports 0)
  (def (walk value)
    (cond
      ((expander-context? value)
       (unless (hash-key? seen value)
         (hash-put! seen value #t)
         (set! total (1+ total))
         (set! bindings (+ bindings (hash-length (expander-context-table value))))
         (when (phi-context? value)
           (walk (phi-context-super value)) (walk (phi-context-up value))
           (walk (phi-context-down value)))
         (cond
           ((module-context? value)
            (set! modules (1+ modules))
            (when (module-context-code value) (set! bodies (1+ bodies)))
            (walk (module-context-import value)) (walk (module-context-export value)))
           ((prelude-context? value)
            (set! preludes (1+ preludes)) (walk (prelude-context-import value))))
         (let (promise (cond ((module-context? value) (module-context-e value))
                              ((prelude-context? value) (prelude-context-e value))
                              (else #f)))
           (when (##promise? promise)
             (let (state (##promise-state promise))
               (if (eq? state (vector-ref state 0))
                 (set! pending (1+ pending)) (set! ready (1+ ready))))))))
      ((module-import? value) (set! imports (1+ imports)) (walk (module-import-source value)))
      ((module-export? value) (set! exports (1+ exports)) (walk (module-export-context value)))
      ((import-set? value) (walk (import-set-source value)) (walk (import-set-imports value)))
      ((export-set? value) (walk (export-set-source value)) (walk (export-set-exports value)))
      ((pair? value) (walk (car value)) (walk (cdr value)))))
  (for-each walk (hash-values __module-registry))
  (u64vector total modules preludes bindings bodies pending ready imports exports))

(def (compiler-roots-snapshot)
  (def registered (make-hash-table-eq))
  (def bindings 0) (def bodies 0)
  (hash-for-each
    (lambda (_key context)
      (when (and (module-context? context) (not (hash-key? registered context)))
        (hash-put! registered context #t)
        (when (module-context-code context) (set! bodies (1+ bodies)))
        (set! bindings (+ bindings (hash-length (expander-context-table context))))))
    __module-registry)
  ;; Import/export counts are visits, not unique objects or allocated bytes.
  (vector (u64vector (hash-length __module-registry) (hash-length registered) bindings bodies)
          (compiler-linkage-topology)))
