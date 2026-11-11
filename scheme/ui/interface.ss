;;; Scheme-owned terminal component composition and focus policy.
(.def Ui
  identity: 'ui
  core: (.o requires: (.o) scope-contract: UiScopeContract exports: UiScope)
  features: (.o))
