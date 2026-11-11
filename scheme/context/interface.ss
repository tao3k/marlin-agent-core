;;; Context operations provided by Marlin to its runtime boundary.

(.def Context
  identity: 'context
  core: (.o requires: (.o LabContext: (.o module-id: 'agentic-ai-lab/modules/ai-agentic-context/interface))
            scope-contract: MarlinContextModuleContract
            exports: MarlinContextScope)
  features: (.o causal: (.o refine: EnableCausalEvidence contract: ContextCausalContract)))
