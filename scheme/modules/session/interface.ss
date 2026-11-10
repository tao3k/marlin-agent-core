;;; Published session policy defaults for the Marlin Scheme VM.

(.def Session
  identity: 'session
  tags: '(marlin)
  core: (.o requires: (.o)
            scope-contract: SessionScopeContract
            exports: SessionScope)
  features: (.o))
