;;; Context operations provided by Marlin to its runtime boundary.

(.def Context
  identity: 'context
  core: (.o requires: (.o)
            scope-contract: MarlinContextModuleContract
            exports: MarlinContextScope)
  features: (.o causal: (.o refine: EnableCausalEvidence contract: ContextCausalContract)))
