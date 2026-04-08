use clap::Parser;
use task_invocation_cron_kernel::cli::TickCli;

#[test]
fn cli_accepts_daemon_subcommand() {
    let parsed = TickCli::try_parse_from(["tick", "daemon"]);
    assert!(
        parsed.is_ok(),
        "expected `tick daemon` to parse, got {parsed:?}"
    );
}

#[test]
fn cli_accepts_add_subcommand_with_required_flags() {
    let parsed = TickCli::try_parse_from([
        "tick",
        "add",
        "--cron",
        "*/5 * * * * *",
        "--role",
        "coder",
        "--cmd",
        "echo hello",
    ]);

    assert!(
        parsed.is_ok(),
        "expected `tick add` to parse, got {parsed:?}"
    );
}
