use clap::Parser;
use task_invocation_cron_kernel::cli::{Mode, TickCli, TickCommand};
use task_invocation_cron_kernel::models::{ExecutionPolicy, JobType};

#[test]
fn cli_builds_default_wasm_job() {
    let cli = TickCli::try_parse_from([
        "tick",
        "add",
        "--cron",
        "*/5 * * * * *",
        "--role",
        "coder",
        "--module",
        "./jobs/task.wasm",
    ])
    .expect("expected add command to parse");

    let TickCommand::Add(add) = cli.command else {
        panic!("expected add command");
    };

    let job = add.into_job().expect("expected add command to build a job");
    assert_eq!(job.policy, ExecutionPolicy::MeshAny);
    assert_eq!(job.mode(), Mode::Wasm);

    match job.job_type {
        JobType::Wasm(wasm) => assert_eq!(wasm.module, "./jobs/task.wasm"),
        other => panic!("expected wasm job, got {other:?}"),
    }
}

#[test]
fn cli_rejects_shell_only_flags_in_wasm_mode() {
    let cli = TickCli::try_parse_from([
        "tick",
        "add",
        "--cron",
        "*/5 * * * * *",
        "--role",
        "coder",
        "--module",
        "./jobs/task.wasm",
        "--command",
        "python3",
    ])
    .expect("expected CLI parse to succeed before semantic validation");

    let TickCommand::Add(add) = cli.command else {
        panic!("expected add command");
    };

    let error = add
        .into_job()
        .expect_err("expected conflicting flags to fail");
    assert!(
        error
            .to_string()
            .contains("--command is only valid with --mode shell"),
        "unexpected error: {error}"
    );
}

#[test]
fn cli_accepts_explicit_shell_mode() {
    let cli = TickCli::try_parse_from([
        "tick",
        "add",
        "--cron",
        "*/5 * * * * *",
        "--role",
        "coder",
        "--mode",
        "shell",
        "--policy",
        "remote-only",
        "--command",
        "python3",
        "--arg",
        "script.py",
    ])
    .expect("expected shell command to parse");

    let TickCommand::Add(add) = cli.command else {
        panic!("expected add command");
    };

    let job = add.into_job().expect("expected valid shell job");
    assert_eq!(job.policy, ExecutionPolicy::RemoteOnly);
    assert_eq!(job.mode(), Mode::Shell);
}
