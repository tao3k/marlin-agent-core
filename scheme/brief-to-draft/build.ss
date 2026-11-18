;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
(import (only-in :clan/poo/object .def .o))

(.def Build
  defbuild-script: (.o spec: '("entry")))
