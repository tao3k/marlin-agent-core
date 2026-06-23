;;; -*- Gerbil -*-
;;; Boundary: downstream sandbox profile selection for the user interface.
;;; Invariant: sandbox construction is owned by poo-flow.

(let ((session-capabilities
       '(process-run filesystem-read tmpdir cache-mount))
      (session-metadata
       '((surface . marlin-user-interface)
         (intent . interactive-policy-handoff)
         (runtime-executed . #f))))
  (use-module nono-sandbox
    (.def (marlin-user-interface/session @ nono-sandbox-profile)
      network: (allowlisted-network "github.com" "crates.io")
      capabilities: session-capabilities
      resources: =>.+ runtime-volume-resources
      metadata: => (lambda (super-metadata)
                     (append super-metadata session-metadata)))))
