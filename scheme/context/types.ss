;;; Contract for Marlin's Context publication.

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/mop Fun Type.)
        (only-in :clan/poo/type Record String OrFalse Exactly Or)
        (only-in :clan/poo/number UInt)
        (only-in :gerbil-ascent/temporal/lens temporal-answer?)
        (only-in :agentic-ai-lab/modules/ai-agentic-context/types AiAgenticContextScopeType))

(export MarlinContextRequest MarlinContextAdmission MarlinContextModuleContract ContextCausalContract)

(def MarlinContextRequest
  (Record bundle: [String] organization: [String] epoch: [UInt]
          project: [String] worktree: [String] source-cut: [String]
          actor: [String] task: [String] destination: [String]
          session: [(OrFalse String)] turn: [(OrFalse UInt)]))

(def MarlinContextAdmission
  (Fun AiAgenticContextScopeType <- MarlinContextRequest))

(def CausalExchange
  (Record kind: [(Exactly 'agentic-ai-lab.ascent-exchange.v1)]))

(def CausalAnswer (.cc Type. '.element? temporal-answer?))

(def CausalReasonRequest (Record exchange: [CausalExchange] root: [String]))
(def CausalReplayRequest (Record exchange: [CausalExchange] root: [String] answer: [CausalAnswer]))
(def CausalReplayVerdict (Or (Exactly 'valid) (Exactly 'invalid)))

(def ContextProjectionRows (.cc Type. '.element? vector?))
(def ContextProjectionBytes (.cc Type. '.element? u8vector?))
(def ContextBridgeContract
  (Record decode-batch: [(Fun ContextProjectionRows <- ContextProjectionRows)]
          encode-scopes: [(Fun ContextProjectionBytes <- ContextProjectionRows)]))

(def MarlinContextModuleContract
  (.cc (Record admit-scope: [MarlinContextAdmission] bridge: [ContextBridgeContract])
       identity: 'marlin/context))

(def ContextCausalContract
  (.cc (Record reason-causally: [(Fun CausalAnswer <- CausalReasonRequest)]
               replay-causal-answer: [(Fun CausalReplayVerdict <- CausalReplayRequest)])
       identity: 'marlin/context/causal))
