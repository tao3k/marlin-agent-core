;;; -*- Gerbil -*-
;;; Engineering note: Support helpers are shared only when they remove repeated
;;; policy-pack mechanics. Keep this module small; domain-specific profile data
;;; belongs in the profile modules so helper reuse does not hide ownership.
package: config-interface/modules

(import :gerbil/gambit
        (only-in :clan/poo/object object<-alist))

(export marlin-vector-map
        marlin-policy-object<-alist
        marlin-string-vector-fingerprint
        marlin-policy-digest)

;;; Boundary: Vector projection keeps module order deterministic for receipts.
;; marlin-vector-map
;;   : (-> Procedure Vector Vector)
;;   | doc m%
;;       `marlin-vector-map map-procedure source-vector` returns a same-size
;;       vector whose elements are mapped in source order.
;;
;;       # Examples
;;       ```scheme
;;       (marlin-vector-map values '#("a" "b"))
;;       ;; => '#("a" "b")
;;       ```
;;     %
(def (marlin-vector-map map-procedure source-vector)
  (let* ((count (vector-length source-vector))
         (mapped (make-vector count)))
    (let loop ((index 0))
      (if (< index count)
        (begin
          (vector-set! mapped
                       index
                       (map-procedure (vector-ref source-vector index)))
          (loop (+ index 1)))
        mapped))))

;;; Boundary: Broad policy records use alist construction to keep POO macro expansion small.
;; marlin-policy-object<-alist
;;   : (-> FieldAlist PolicyObject)
;;   | doc m%
;;       `marlin-policy-object<-alist field-values` builds wide policy records
;;       from explicit key/value pairs instead of expanding broad `.o` forms.
;;
;;       # Examples
;;       ```scheme
;;       (marlin-policy-object<-alist (list (cons 'kind "policy")))
;;       ;; => policy object
;;       ```
;;     %
(def (marlin-policy-object<-alist field-values)
  (object<-alist field-values))

;;; Boundary: Policy digests are derived from typed policy inputs, not fixtures.
;; marlin-string-vector-fingerprint
;;   : (-> StringVector String)
;;   | doc m%
;;       `marlin-string-vector-fingerprint source-vector` creates a stable
;;       pipe-prefixed fingerprint for ordered string slots.
;;
;;       # Examples
;;       ```scheme
;;       (marlin-string-vector-fingerprint '#("hot" "audit"))
;;       ;; => "|hot|audit"
;;       ```
;;     %
(def (marlin-string-vector-fingerprint source-vector)
  (apply string-append
         (cons ""
               (map (lambda (value)
                      (string-append "|" value))
                    (vector->list source-vector)))))

;;; Boundary: Fixed-size digest mutation avoids allocating an intermediate byte list.
;; marlin-policy-digest
;;   : (-> String Integer StringVector StringVector String String ByteVector)
;;   | doc m%
;;       `marlin-policy-digest profile epoch mechanisms mixins hot audit`
;;       returns the deterministic 32-byte policy digest used by receipts.
;;
;;       # Examples
;;       ```scheme
;;       (vector-length (marlin-policy-digest "default" 1 '#() '#() "" ""))
;;       ;; => 32
;;       ```
;;     %
(def (marlin-policy-digest profile-id-value
                           policy-epoch-value
                           mechanism-policies-value
                           policy-mixins-value
                           hot-fingerprint-value
                           audit-fingerprint-value)
  (let* ((seed
          (string-append profile-id-value
                         ":epoch=" (number->string policy-epoch-value)
                         ":mechanisms="
                         (marlin-string-vector-fingerprint mechanism-policies-value)
                         ":mixins="
                         (marlin-string-vector-fingerprint policy-mixins-value)
                         ":hot=" hot-fingerprint-value
                         ":audit=" audit-fingerprint-value))
         (seed-length (string-length seed))
         (digest (make-vector 32 0)))
    (let loop ((index 0)
               (state (modulo (+ seed-length policy-epoch-value) 256)))
      (if (< index 32)
        (let* ((seed-byte
                (char->integer (string-ref seed (modulo index seed-length))))
               (next-byte
                (modulo (+ state
                           seed-byte
                           (* (+ index 1) 17)
                           (* policy-epoch-value 13))
                        256)))
          (vector-set! digest index next-byte)
          (loop (+ index 1) next-byte))
        digest))))
