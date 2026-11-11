;;; Publish the Lab-owned Org Contract source cut without a Rust Org parser.
(import (only-in :clan/poo/object .ref)
        (only-in :agentic-ai-lab/modules/ai-agentic-context/org-contract/elements
                 context-org-source))
(export marlin-context-org-contract-source)

(def (marlin-context-org-contract-source request)
  (context-org-source (.ref request 'identity) (.ref request 'generation)
                      (.ref request 'source)))
