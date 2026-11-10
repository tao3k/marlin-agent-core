;;; Policy operations exposed through Marlin's Yuanxu selection.

(.def Policy
  identity: 'policy
  core: (.o requires: (.o)
            scope-contract: PolicyScopeContract
            exports: PolicyScope)
  features: (.o))
