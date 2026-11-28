// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! SM39 (ADV-L4) TUI Config, ASP Supervisor, Diagnostic Probe & Bootstrap Confinement tests.

use std::fs;
use std::time::Duration;

use crate::config::Config;

#[test]
fn sm39_tui_config_refuses_symlink_oversize_bad_timeout_and_ambient_credential_env() {
    let dir = tempfile::tempdir().expect("tempdir");

    // 1. Oversize TOML (> 256 KiB) must be refused before parsing.
    let big_path = dir.path().join("giant.toml");
    let big_body = format!(
        "provider = \"openai\"\nmodel = \"gpt-4o\"\nendpoint = \"http://127.0.0.1:1234/v1\"\nprotocol = \"open-ai\"\ncredential_env = \"OPENAI_API_KEY\"\n# {}\n",
        "x".repeat(300 * 1024)
    );
    fs::write(&big_path, big_body).expect("write giant config");
    assert!(
        Config::load(&big_path).is_err(),
        "oversize (> 256 KiB) TUI config must fail closed"
    );

    // 2. Symlinked config path must be refused.
    #[cfg(unix)]
    {
        let target = dir.path().join("real.toml");
        fs::write(
            &target,
            "provider = \"openai\"\nmodel = \"gpt-4o\"\nendpoint = \"http://127.0.0.1:1234/v1\"\nprotocol = \"open-ai\"\ncredential_env = \"OPENAI_API_KEY\"\n",
        )
        .expect("write target config");
        let link = dir.path().join("link.toml");
        std::os::unix::fs::symlink(&target, &link).expect("create symlink");
        assert!(
            Config::load(&link).is_err(),
            "symlinked TUI config file must fail closed"
        );
    }

    // 3. Zero and runaway (> 600s) timeout_seconds must fail closed.
    for bad_timeout in [0u64, 86_400] {
        let raw = format!(
            "provider = \"openai\"\nmodel = \"gpt-4o\"\nendpoint = \"http://127.0.0.1:1234/v1\"\nprotocol = \"open-ai\"\ncredential_env = \"OPENAI_API_KEY\"\ntimeout_seconds = {bad_timeout}\n"
        );
        let cfg: Config = toml::from_str(&raw).expect("parse toml");
        assert!(
            cfg.catalog_with(|k| (k == "OPENAI_API_KEY").then(|| "tok".to_owned()))
                .is_err(),
            "out-of-range timeout_seconds={bad_timeout} must fail closed"
        );
    }

    // 4. Ambient OS/loader/SSH/Git credential_env names must fail closed.
    for hostile_env in [
        "HOME",
        "PATH",
        "LD_PRELOAD",
        "DYLD_INSERT_LIBRARIES",
        "SSH_AUTH_SOCK",
        "GIT_ASKPASS",
        "BAD KEY",
    ] {
        let raw = format!(
            "provider = \"openai\"\nmodel = \"gpt-4o\"\nendpoint = \"http://127.0.0.1:1234/v1\"\nprotocol = \"open-ai\"\ncredential_env = \"{hostile_env}\"\n"
        );
        let cfg: Config = toml::from_str(&raw).expect("parse toml");
        assert!(
            cfg.catalog_with(|_| Some("ambient-secret-value".to_owned()))
                .is_err(),
            "ambient or malformed credential_env={hostile_env:?} must fail closed"
        );
    }
}

#[cfg(unix)]
fn is_pid_alive(pid: i32) -> bool {
    if pid <= 1 {
        return false;
    }
    // SAFETY: signal 0 probes process existence without delivering a signal.
    let rc = unsafe { libc::kill(pid, 0) };
    if rc == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
}

#[cfg(unix)]
#[test]
fn sm39_asp_supervisor_scrubs_host_secrets_and_rejects_spoofed_endpoint() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");

    // 1. Host secret canary must NEVER leak into spawned asp-gateway child env.
    let leak_file = dir.path().join("leaked_env.txt");
    let canary_script = dir.path().join("canary-asp-gateway.sh");
    fs::write(
        &canary_script,
        format!(
            "#!/bin/sh\nprintf '%s' \"$MARLIN_SM39_HOST_SECRET_CANARY\" > {}\nprintf '{{\"endpoint\":\"http://127.0.0.1:1\",\"providers\":[]}}\\n'\nsleep 5\n",
            leak_file.display()
        ),
    )
    .expect("write canary script");
    let mut perms = fs::metadata(&canary_script).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&canary_script, perms).expect("chmod");

    // SAFETY: test-unique canary env var scoped to this test.
    unsafe {
        std::env::set_var("MARLIN_SM39_HOST_SECRET_CANARY", "top-secret-sm39-canary");
    }
    let sup = crate::app::asp_supervisor::spawn_managed_asp_gateway(&canary_script);
    unsafe {
        std::env::remove_var("MARLIN_SM39_HOST_SECRET_CANARY");
    }
    assert!(sup.is_some(), "clean loopback mock gateway should start");
    drop(sup);

    let leaked = fs::read_to_string(&leak_file).unwrap_or_default();
    assert!(
        leaked.is_empty(),
        "ambient host secret leaked into asp-gateway child process: {leaked:?}"
    );

    // 2. Spoofed @-userinfo endpoint in child boot JSON must fail closed & reap child.
    let spoof_pid_file = dir.path().join("spoof.pid");
    let spoof_script = dir.path().join("spoof-asp-gateway.sh");
    fs::write(
        &spoof_script,
        format!(
            "#!/bin/sh\necho $$ > {}\nprintf '{{\"endpoint\":\"http://127.0.0.1:80@evil.example\",\"controlToken\":\"tok\",\"providers\":[]}}\\n'\nsleep 30\n",
            spoof_pid_file.display()
        ),
    )
    .expect("write spoof script");
    let mut perms = fs::metadata(&spoof_script).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&spoof_script, perms).expect("chmod");

    assert!(
        crate::app::asp_supervisor::spawn_managed_asp_gateway(&spoof_script).is_none(),
        "spoofed @-userinfo ASP gateway endpoint must fail closed"
    );
    let pid: i32 = fs::read_to_string(&spoof_pid_file)
        .expect("spoof pid")
        .trim()
        .parse()
        .expect("pid int");
    let alive = is_pid_alive(pid);
    assert!(
        !alive,
        "child PID {pid} must be reaped after spoofed endpoint refusal"
    );

    // 3. Silent/wedged asp-gateway that never writes a boot line must time out cleanly & reap child.
    let hang_pid_file = dir.path().join("hang.pid");
    let hang_script = dir.path().join("hang-asp-gateway.sh");
    fs::write(
        &hang_script,
        format!("#!/bin/sh\necho $$ > {}\nsleep 30\n", hang_pid_file.display()),
    )
    .expect("write hang script");
    let mut perms = fs::metadata(&hang_script).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&hang_script, perms).expect("chmod");

    let started = std::time::Instant::now();
    assert!(
        crate::app::asp_supervisor::spawn_managed_asp_gateway_with_timeout(
            &hang_script,
            Duration::from_millis(200),
        )
        .is_none(),
        "wedged asp-gateway must time out fail-closed"
    );
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "boot timeout should abort promptly instead of hanging"
    );
    if let Ok(raw_pid) = fs::read_to_string(&hang_pid_file)
        && let Ok(hang_pid) = raw_pid.trim().parse::<i32>()
    {
        let hang_alive = is_pid_alive(hang_pid);
        assert!(
            !hang_alive,
            "wedged child PID {hang_pid} must be reaped after boot timeout"
        );
    }

    // 4. Relative PATH entries (like ".") must never be searched for asp-gateway binaries.
    let fake_cwd = tempfile::tempdir().expect("fake cwd");
    let fake_bin = fake_cwd.path().join("asp-gateway");
    fs::write(&fake_bin, "#!/bin/sh\nexit 0\n").expect("write fake bin");
    let mut perms = fs::metadata(&fake_bin).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&fake_bin, perms).expect("chmod");
    let discovered = crate::app::asp_supervisor::discover_asp_gateway_binary_with_path(
        fake_cwd.path(),
        Some(std::ffi::OsStr::new(".")),
    );
    assert!(
        discovered.as_deref() != Some(std::path::Path::new("./asp-gateway")),
        "relative PATH entry '.' must never resolve ./asp-gateway"
    );
}

#[cfg(unix)]
#[test]
fn sm39_runtime_probe_refuses_symlink_target_overwrite() {
    let dir = tempfile::tempdir().expect("tempdir");
    let victim = dir.path().join("victim.txt");
    fs::write(&victim, "DO_NOT_OVERWRITE").expect("write victim");
    let symlink_metrics = dir.path().join("metrics-link.csv");
    std::os::unix::fs::symlink(&victim, &symlink_metrics).expect("create symlink");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio rt");
    let (owner, _events) = marlin_agent_runtime::TokioAgentRuntime::new(8);

    // 1. Symlink target must never be overwritten.
    let probe = crate::app::runtime_probe::RuntimeProbe::start_to_path(
        &rt,
        &owner,
        symlink_metrics,
        crate::app::runtime_probe::MAX_RUNTIME_PROBE_BYTES,
    );
    std::thread::sleep(Duration::from_millis(60));
    drop(probe);

    let after = fs::read_to_string(&victim).expect("read victim");
    assert_eq!(
        after, "DO_NOT_OVERWRITE",
        "RuntimeProbe followed symlink and overwrote victim file with {after:?}"
    );

    // 2. Cumulative written bytes must obey max_bytes cap cleanly.
    let bounded_metrics = dir.path().join("bounded.csv");
    let capped = crate::app::runtime_probe::RuntimeProbe::start_to_path(
        &rt,
        &owner,
        bounded_metrics.clone(),
        16,
    );
    std::thread::sleep(Duration::from_millis(320));
    drop(capped);
    let written = fs::read_to_string(&bounded_metrics).expect("read bounded");
    assert!(
        written.len() <= 16 && !written.is_empty(),
        "RuntimeProbe must stop writing cleanly once max_bytes cap is reached; got {} bytes",
        written.len()
    );
}

#[cfg(unix)]
#[test]
fn sm39_bootstrap_prioritizes_binary_repo_over_untrusted_cwd_and_refuses_symlinks() {
    let trusted_repo = tempfile::tempdir().expect("trusted repo");
    let untrusted_cwd = tempfile::tempdir().expect("untrusted cwd");

    let seed = |root: &std::path::Path| {
        let pkg = root.join("scheme");
        let prefix = pkg.join(".gerbil");
        fs::create_dir_all(prefix.join("lib/marlin-agent-core")).expect("mkdir");
        fs::write(pkg.join("gerbil.pkg"), "(package: marlin-agent-core)\n").expect("pkg");
        fs::write(prefix.join("lib/marlin-agent-core/init.ssi"), "prelude:\n").expect("init");
        prefix
    };

    let trusted_prefix = seed(trusted_repo.path());
    let _hostile_cwd_prefix = seed(untrusted_cwd.path());
    let exe = trusted_repo.path().join("target/debug/marlin-tui");

    // 1. Binary's own repo MUST win over an untrusted CWD ./scheme directory.
    let resolved = crate::bootstrap::resolve_local_scheme_prefix(
        None,
        untrusted_cwd.path(),
        Some(&exe),
    );
    assert_eq!(
        resolved.as_deref(),
        Some(trusted_prefix.as_path()),
        "untrusted CWD ./scheme hijacked compiled binary's own repository prefix"
    );

    // 2. Symlinked ./scheme directory in CWD must be refused even when exe has no repo.
    let sym_cwd = tempfile::tempdir().expect("sym cwd");
    std::os::unix::fs::symlink(trusted_repo.path().join("scheme"), sym_cwd.path().join("scheme"))
        .expect("symlink scheme");
    assert_eq!(
        crate::bootstrap::resolve_local_scheme_prefix(None, sym_cwd.path(), None),
        None,
        "symlinked CWD ./scheme must be refused"
    );
}
