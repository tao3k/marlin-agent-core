;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Metadata does not duplicate selected scopes or grant native readiness.
(import (only-in :clan/poo/object .o))
(export CAbiMetadata)
(def CAbiMetadata (.o identity: 'marlin/c-abi))
