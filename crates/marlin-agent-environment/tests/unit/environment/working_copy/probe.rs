// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_environment::{
    ProcessWorkingCopyCommandRunner, WorkingCopyProviderExecutableStatus,
};
use marlin_agent_gitops::working_copy::WorkingCopyCommandProgram;

#[tokio::test]
async fn process_runner_can_probe_git_executable_without_mutating_workspace() {
    let probe = ProcessWorkingCopyCommandRunner::probe_program(WorkingCopyCommandProgram::Git)
        .await
        .expect("git probe should run");

    assert_eq!(probe.program, WorkingCopyCommandProgram::Git);
    assert_eq!(probe.executable, "git");
    assert_eq!(probe.status, WorkingCopyProviderExecutableStatus::Available);
    assert_eq!(probe.status_code, Some(0));
    assert!(
        probe
            .stdout
            .as_deref()
            .or(probe.stderr.as_deref())
            .is_some_and(|output| output.contains("git"))
    );
}

#[tokio::test]
#[ignore = "requires globally installed wt"]
async fn process_runner_can_probe_global_worktrunk_executable() {
    let probe =
        ProcessWorkingCopyCommandRunner::probe_program(WorkingCopyCommandProgram::Worktrunk)
            .await
            .expect("wt probe should run");

    assert_eq!(probe.program, WorkingCopyCommandProgram::Worktrunk);
    assert_eq!(probe.executable, "wt");
    assert_eq!(probe.status, WorkingCopyProviderExecutableStatus::Available);
    assert_eq!(probe.status_code, Some(0));
    assert!(
        probe
            .stdout
            .as_deref()
            .or(probe.stderr.as_deref())
            .is_some_and(|output| output.contains("wt"))
    );
}
