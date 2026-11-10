;;; Project Marlin's boundary arguments into Lab's Scope library.

(import (only-in :clan/poo/object .o .get)
        (only-in :agentic-ai-lab/modules/ai-agentic-context/objects AiAgenticContextScope.)
        (only-in :agentic-ai-lab/modules/ai-agentic-context/funs-scope
                 agentic-ai-lab-ai-agentic-context-scope-admit))

(export marlin-context-admit-scope)

(def (marlin-context-admit-scope request)
  (agentic-ai-lab-ai-agentic-context-scope-admit
    (.o (:: @ AiAgenticContextScope.)
        bundle: (.get request bundle)
        organization: (.get request organization)
        epoch: (.get request epoch)
        project: (.get request project)
        worktree: (.get request worktree)
        source-cut: (.get request source-cut)
        actor: (.get request actor)
        task: (.get request task)
        destination: (.get request destination)
        session: (.get request session)
        turn: (.get request turn))))
