// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// MAR-PROC-R18: observer evidence and Runtime effects are separate.
module hostObserver {
  type State = { event: str, hostAlive: bool, toolAlive: bool, tui: str, tokio: str, exitObserved: bool }
  var s: State

  action init = s' = { event: "attach", hostAlive: true, toolAlive: false, tui: "started", tokio: "started", exitObserved: false }
  action startTool = all {
    s.event == "attach",
    s' = { ...s, event: "start", toolAlive: true },
  }
  action stopHost = all {
    s.event == "start",
    s' = { ...s, event: "stop", tui: "missed", tokio: "missed" },
  }
  action resumeHost = all {
    s.event == "stop",
    s' = { ...s, event: "resume", tui: "resumed", tokio: "resumed" },
  }
  action quitHost = all {
    s.event == "resume",
    s' = { ...s, event: "quit", hostAlive: false, toolAlive: false, exitObserved: true },
  }
  action step = any { startTool, stopHost, resumeHost, quitHost, all { s.event == "quit", s' = s } }
  action badStop = all {
    s.event == "start",
    s' = { ...s, event: "stop", tui: "missed", tokio: "missed", exitObserved: true, toolAlive: false },
  }
  action badStep = any { startTool, badStop, resumeHost, quitHost, all { s.event == "quit", s' = s } }
  val safe = all {
    s.exitObserved == not(s.hostAlive),
    s.tui == "missed" implies s.toolAlive,
  }
  run pauseThenQuitTest = init.then(startTool).then(stopHost).then(resumeHost).then(quitHost)
}
