;;; Hook decisions published by Marlin.

(.def Hooks
  identity: 'hooks
  core: (.o requires: (.o)
            scope-contract: HooksScopeContract
            exports: HooksScope)
  features: (.o))
