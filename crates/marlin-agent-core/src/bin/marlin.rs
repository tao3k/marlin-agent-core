fn main() -> std::process::ExitCode {
    let result = marlin_agent_core::run_marlin_cli();
    if !result.stdout.is_empty() {
        print!("{}", result.stdout);
    }
    if !result.stderr.is_empty() {
        eprint!("{}", result.stderr);
    }
    std::process::ExitCode::from(result.status as u8)
}
