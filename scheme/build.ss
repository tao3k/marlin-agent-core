#!/usr/bin/env gxi
;;; The package roles define the source set for Gerbil's standard builder.

(import (only-in :std/build-script defbuild-script)
        (only-in :std/source this-source-file))

(export main)

(def package-root (path-directory (this-source-file)))

(def (role-sources role)
  (map (lambda (file) (string-append role "/" file))
       (list-sort string<?
                  (filter (lambda (file) (equal? (path-extension file) ".ss"))
                          (directory-files (path-expand role package-root))))))

;; Yuanxu admits the selected publications from init.ss. The Rust build
;; links c-abi/ separately because those files export C entry points.
(defbuild-script (cons "init.ss" (append (role-sources "policy")
                                       (role-sources "projection"))))
