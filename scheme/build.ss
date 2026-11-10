#!/usr/bin/env gxi
;;; The selected Yuanxu modules own the Gerbil library build.

(import (for-syntax
          (only-in :std/source this-source-file)
          (only-in :yuanxu/module-system/building/source-package
                   register-module-build-source!)))

(begin-syntax
  (register-module-build-source! (path-directory (this-source-file))
                                 'marlin-agent-core))

(import (only-in :clan/poo/object .get)
        (only-in :yuanxu/module-system/building/script defbuild-script)
        (only-in "./init.ss" marlin-agent-core.init.exports))

(export main)

(def modules-spec (.get marlin-agent-core.init.exports std-make-spec))
(defbuild-script (modules-spec '("init")))
