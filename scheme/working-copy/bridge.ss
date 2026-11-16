;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports))

(export select-working-copy-provider)

(def (select-working-copy-provider request)
  ((.get marlin-agent-core.init.exports Marlin working-copy select-provider) request))
