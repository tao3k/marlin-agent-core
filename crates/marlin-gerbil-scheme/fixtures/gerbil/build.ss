#!/usr/bin/env gxi
;;; -*- Gerbil -*-
;;; Build script for the Marlin Gerbil runtime assets.

(import :std/build-script)

(defrules defmarlin-runtime-build-script ()
  ((_ extra-target ...)
   (defbuild-script
     '("marlin/protocol"
       "marlin/request"
       "marlin/parser"
       "marlin/adapter"
       extra-target ...))))

(defmarlin-runtime-build-script
  (exe: "command-adapter"))
