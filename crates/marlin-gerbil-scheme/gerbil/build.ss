#!/usr/bin/env gxi
;;; -*- Gerbil -*-
;;; Build script for the Marlin Gerbil runtime assets.

(import :std/build-script)

(defrules defmarlin-runtime-build-script ()
  ((_ extra-target ...)
   (defbuild-script
     '((gsc: "src/marlin/deck-runtime-native")
       (ssi: "src/marlin/deck-runtime-native")
       "src/marlin/protocol"
       "src/marlin/request"
       "src/marlin/parser"
       "src/marlin/adapter"
       "src/marlin/deck-runtime"
       "src/marlin/deck-runtime-compiled-policy"
       "src/marlin/deck-runtime-compiled-policy-sample"
       "src/marlin/deck-runtime-strategy"
       extra-target ...))))

(defmarlin-runtime-build-script
  (exe: "bin/command-adapter")
  (exe: "bin/command-adapter-batch"))
