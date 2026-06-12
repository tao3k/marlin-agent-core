#!/usr/bin/env gxi
;;; -*- Gerbil -*-
;;; Build script for the Marlin Gerbil runtime assets.

(import :std/build-script)

(defrules defmarlin-runtime-build-script ()
  ((_ extra-target ...)
   (defbuild-script
     '("src/marlin/protocol"
       "src/marlin/request"
       "src/marlin/parser"
       "src/marlin/adapter"
       "src/marlin/hook-policy"
       extra-target ...))))

(defmarlin-runtime-build-script
  (exe: "bin/command-adapter")
  (exe: "bin/command-adapter-batch")
  (exe: "bin/hook-policy-adapter"))
