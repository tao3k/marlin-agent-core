;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Modular Repair publication with opt-in `+ast-patch` feature (`scheme/repair`).
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" RepairScope)
        (only-in "./ast-patch.ss" EnableAstPatchRepair)
        (only-in "./types.ss" RepairScopeContract RepairAstPatchContract))
(export Repair)

(.def Repair
  identity: 'repair
  core: (.o scope-contract: RepairScopeContract
            exports: RepairScope)
  features: (.o ast-patch: (.o refine: EnableAstPatchRepair
                               contract: RepairAstPatchContract)))
