;;; -*- Gerbil -*-
;;; Executable source-file entry point for batched Marlin Gerbil command requests.

(import (only-in :marlin/adapter run-marlin-command-adapter-batch))

(run-marlin-command-adapter-batch)
