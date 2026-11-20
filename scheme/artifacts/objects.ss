;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .def .o))
(export ArtifactKinds)

(.def ArtifactKinds
  git-patch: (.o kind: 'git-patch)
  test-receipt: (.o kind: 'test-receipt))
