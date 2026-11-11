#!/usr/bin/env gxi
;;; Build the package selected by Yuanxu.

(import (only-in :clan/poo/object .get)
        (only-in :yuanxu/module-system/building/script defbuild-script)
        (only-in "./init.ss" marlin-agent-core.init.exports))

(export main)

(defbuild-script ((.get marlin-agent-core.init.exports std-make-spec) '("init" "c-abi/deck-runtime")))
