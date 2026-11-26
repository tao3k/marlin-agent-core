// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// MAR-UI-R26: TUI bootstrap, stdin EINTR/EOF custody, terminal restoration,
// and bounded shutdown invariants across linked and dynamic builds.
module tuiLifecycle {
  type State = {
    phase: str,
    linkedUi: bool,
    gerbilPathValid: bool,
    terminalRaw: bool,
    firstFrameDrawn: bool,
    stdinAlive: bool,
    eintrCount: int,
    resizeCount: int,
    envrcLoading: bool,
    warmIndexRunning: bool,
    shutdownSteps: int,
    exitReason: str,
    exitCode: int,
    diagnosticEmitted: bool,
  }

  var s: State

  pure val maxEintr = 3
  pure val maxResize = 3
  pure val maxShutdownSteps = 2

  action init = {
    nondet linked = Set(true, false).oneOf()
    nondet validPath = Set(true, false).oneOf()
    s' = {
      phase: "bootstrap",
      linkedUi: linked,
      gerbilPathValid: if (linked) true else validPath,
      terminalRaw: false,
      firstFrameDrawn: false,
      stdinAlive: false,
      eintrCount: 0,
      resizeCount: 0,
      envrcLoading: false,
      warmIndexRunning: false,
      shutdownSteps: 0,
      exitReason: "none",
      exitCode: -1,
      diagnosticEmitted: false,
    }
  }

  action initDynamicPolluted = s' = {
    phase: "bootstrap",
    linkedUi: false,
    gerbilPathValid: false,
    terminalRaw: false,
    firstFrameDrawn: false,
    stdinAlive: false,
    eintrCount: 0,
    resizeCount: 0,
    envrcLoading: false,
    warmIndexRunning: false,
    shutdownSteps: 0,
    exitReason: "none",
    exitCode: -1,
    diagnosticEmitted: false,
  }

  action initLinked = s' = {
    phase: "bootstrap",
    linkedUi: true,
    gerbilPathValid: true,
    terminalRaw: false,
    firstFrameDrawn: false,
    stdinAlive: false,
    eintrCount: 0,
    resizeCount: 0,
    envrcLoading: false,
    warmIndexRunning: false,
    shutdownSteps: 0,
    exitReason: "none",
    exitCode: -1,
    diagnosticEmitted: false,
  }

  // Dynamic bootstrap detects a polluted or missing GERBIL_PATH and selects
  // the workspace scheme/.gerbil prefix before initializing Gambit.
  action resolveLocalPackage = all {
    s.phase == "bootstrap",
    not(s.linkedUi),
    not(s.gerbilPathValid),
    s' = { ...s, gerbilPathValid: true },
  }

  action failMissingPackage = all {
    s.phase == "bootstrap",
    not(s.linkedUi),
    not(s.gerbilPathValid),
    s' = {
      ...s,
      phase: "exited",
      exitReason: "bootstrap-error",
      exitCode: 1,
      diagnosticEmitted: true,
    },
  }

  action openTerminal = all {
    s.phase == "bootstrap",
    s.linkedUi or s.gerbilPathValid,
    s' = {
      ...s,
      phase: "opening",
      terminalRaw: true,
      stdinAlive: true,
    },
  }

  action renderFirstFrame = all {
    s.phase == "opening",
    s.terminalRaw,
    s.stdinAlive,
    s' = {
      ...s,
      phase: "running",
      firstFrameDrawn: true,
      envrcLoading: true,
      warmIndexRunning: true,
    },
  }

  action envrcComplete = all {
    s.phase == "running",
    s.envrcLoading,
    s' = { ...s, envrcLoading: false },
  }

  action terminalResize = all {
    s.phase == "running",
    s.stdinAlive,
    s.resizeCount < maxResize,
    s' = { ...s, resizeCount: s.resizeCount + 1 },
  }

  action warmIndexFinish = all {
    s.phase == "running",
    s.warmIndexRunning,
    s' = { ...s, warmIndexRunning: false },
  }

  // Handled signals such as SIGWINCH may interrupt stdin reads with EINTR.
  // RetryInterruptedRead retries transparently without terminating stdin.
  action handleEintr = all {
    s.phase == "opening" or s.phase == "running",
    s.stdinAlive,
    s.eintrCount < maxEintr,
    s' = { ...s, eintrCount: s.eintrCount + 1 },
  }

  action suspendTerminal = all {
    s.phase == "running",
    s.terminalRaw,
    s' = { ...s, phase: "suspended", terminalRaw: false },
  }

  action resumeTerminal = all {
    s.phase == "suspended",
    not(s.terminalRaw),
    s' = { ...s, phase: "running", terminalRaw: true },
  }

  action userQuit = all {
    s.phase == "running",
    s' = {
      ...s,
      phase: "draining",
      terminalRaw: false,
      exitReason: "user-quit",
      exitCode: 0,
      shutdownSteps: 1,
    },
  }

  action signalExit = all {
    s.phase == "opening" or s.phase == "running" or s.phase == "suspended",
    s' = {
      ...s,
      phase: "draining",
      terminalRaw: false,
      exitReason: "signal",
      exitCode: 130,
      shutdownSteps: 1,
    },
  }

  action stdinEof = all {
    s.phase == "opening" or s.phase == "running",
    s.stdinAlive,
    s' = {
      ...s,
      phase: "draining",
      stdinAlive: false,
      terminalRaw: false,
      exitReason: "stdin-eof",
      exitCode: 1,
      diagnosticEmitted: true,
      shutdownSteps: 1,
    },
  }

  action stdinIoError = all {
    s.phase == "opening" or s.phase == "running",
    s.stdinAlive,
    s' = {
      ...s,
      phase: "draining",
      stdinAlive: false,
      terminalRaw: false,
      exitReason: "input-error",
      exitCode: 1,
      diagnosticEmitted: true,
      shutdownSteps: 1,
    },
  }

  // Runtime shutdown uses a bounded timeout so background index warming
  // cannot hang process exit after tracked tasks are drained.
  action completeBoundedShutdown = all {
    s.phase == "draining",
    s.shutdownSteps < maxShutdownSteps,
    s' = {
      ...s,
      phase: "exited",
      warmIndexRunning: false,
      shutdownSteps: s.shutdownSteps + 1,
    },
  }

  action stutterExited = all {
    s.phase == "exited",
    s' = s,
  }

  action step = any {
    resolveLocalPackage,
    failMissingPackage,
    openTerminal,
    renderFirstFrame,
    envrcComplete,
    terminalResize,
    warmIndexFinish,
    handleEintr,
    suspendTerminal,
    resumeTerminal,
    userQuit,
    signalExit,
    stdinEof,
    stdinIoError,
    completeBoundedShutdown,
    stutterExited,
  }

  // Negative controls: each mutation reintroduces a concrete bug class.
  action badOpenWithPollutedGerbilPath = all {
    s.phase == "bootstrap",
    not(s.linkedUi),
    not(s.gerbilPathValid),
    s' = { ...s, phase: "opening", terminalRaw: true, stdinAlive: true },
  }

  action pollutedGerbilPathMutation = any {
    badOpenWithPollutedGerbilPath,
    step,
  }

  action badEintrDropsStdin = all {
    s.phase == "opening" or s.phase == "running",
    s.stdinAlive,
    s' = { ...s, stdinAlive: false, eintrCount: s.eintrCount + 1 },
  }

  action eintrBreaksStdinMutation = any {
    badEintrDropsStdin,
    step,
  }

  action badSilentEofExit = all {
    s.phase == "opening" or s.phase == "running",
    s.stdinAlive,
    s' = {
      ...s,
      phase: "draining",
      stdinAlive: false,
      terminalRaw: false,
      exitReason: "stdin-eof",
      exitCode: 0,
      diagnosticEmitted: false,
      shutdownSteps: 1,
    },
  }

  action silentEofExitMutation = any {
    badSilentEofExit,
    step,
  }

  action badStrandedTerminalOnSignal = all {
    s.phase == "running",
    s.terminalRaw,
    s' = {
      ...s,
      phase: "draining",
      terminalRaw: true,
      exitReason: "signal",
      exitCode: 130,
      shutdownSteps: 1,
    },
  }

  action strandedTerminalMutation = any {
    badStrandedTerminalOnSignal,
    step,
  }

  action badUnboundedShutdownHang = all {
    s.phase == "draining",
    s.warmIndexRunning,
    s' = { ...s, shutdownSteps: s.shutdownSteps + 1 },
  }

  action unboundedShutdownHangMutation = any {
    badUnboundedShutdownHang,
    step,
  }

  action badSpontaneousSteadyStateExit = all {
    s.phase == "running",
    s.envrcLoading,
    s' = {
      ...s,
      phase: "draining",
      terminalRaw: false,
      exitReason: "unknown",
      exitCode: 0,
      shutdownSteps: 1,
    },
  }

  action spontaneousExitMutation = any {
    badSpontaneousSteadyStateExit,
    step,
  }

  val bootstrapPackageValid =
    (s.phase != "bootstrap" and s.exitReason != "bootstrap-error") implies
      (s.linkedUi or s.gerbilPathValid)

  val terminalRestoredOnExit =
    (s.phase == "draining" or s.phase == "exited" or s.phase == "suspended") implies
      not(s.terminalRaw)

  val eintrNeverKillsStdin =
    (s.phase == "opening" or s.phase == "running") implies s.stdinAlive

  val eofNeverSilentSuccess =
    s.exitReason == "stdin-eof" implies (s.exitCode != 0 and s.diagnosticEmitted)

  val errorsAlwaysDiagnosed =
    (s.exitReason == "stdin-eof" or s.exitReason == "input-error" or s.exitReason == "bootstrap-error") implies
      (s.exitCode == 1 and s.diagnosticEmitted)

  val cleanExitOnlyOnUserQuit =
    s.exitCode == 0 implies s.exitReason == "user-quit"

  val boundedShutdown = s.shutdownSteps <= maxShutdownSteps

  val exitedClean =
    s.phase == "exited" implies all {
      not(s.terminalRaw),
      not(s.warmIndexRunning),
      s.exitReason != "none",
      s.exitCode >= 0,
    }

  val safe = all {
    bootstrapPackageValid,
    terminalRestoredOnExit,
    eintrNeverKillsStdin,
    eofNeverSilentSuccess,
    errorsAlwaysDiagnosed,
    cleanExitOnlyOnUserQuit,
    boundedShutdown,
    exitedClean,
  }

  run dynamicPollutedPathRecoveryTest =
    initDynamicPolluted
      .then(resolveLocalPackage)
      .then(openTerminal)
      .then(renderFirstFrame)
      .then(userQuit)
      .then(completeBoundedShutdown)
      .then(all { assert(safe and s.phase == "exited" and s.exitCode == 0), s' = s })

  run eintrAndSuspendThenQuitTest =
    initLinked
      .then(openTerminal)
      .then(renderFirstFrame)
      .then(handleEintr)
      .then(suspendTerminal)
      .then(resumeTerminal)
      .then(handleEintr)
      .then(userQuit)
      .then(completeBoundedShutdown)
      .then(all { assert(safe and s.eintrCount == 2 and s.exitCode == 0), s' = s })

  run stdinEofDiagnosedTest =
    initLinked
      .then(openTerminal)
      .then(renderFirstFrame)
      .then(stdinEof)
      .then(completeBoundedShutdown)
      .then(all { assert(safe and s.exitCode == 1 and s.diagnosticEmitted and not(s.terminalRaw)), s' = s })

  run boundedShutdownWhileWarmingTest =
    initLinked
      .then(openTerminal)
      .then(renderFirstFrame)
      .then(userQuit)
      .then(completeBoundedShutdown)
      .then(all { assert(safe and not(s.warmIndexRunning) and s.shutdownSteps <= maxShutdownSteps), s' = s })
}
