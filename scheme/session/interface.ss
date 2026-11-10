;;; Published session policy defaults for the Marlin Scheme VM.

(.def Session
  identity: 'session
  core: (.o requires: (.o)
            scope-contract: SessionScopeContract
            exports: SessionScope)
  features: (.o))
