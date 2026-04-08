use clap::Parser;
use task_invocation_cron_kernel::cli::{TickCli, TickCommand};

#[test]
fn cli_accepts_list_with_json_flag() {
    let cli = TickCli::try_parse_from(["tick", "list", "--json"])
        .expect("expected list command to parse");

    match cli.command {
        TickCommand::List => assert!(cli.json),
        other => panic!("expected list command, got {other:?}"),
    }
}

#[test]
fn cli_accepts_inspect_with_job_id() {
    let cli = TickCli::try_parse_from([
        "tick",
        "inspect",
        "550e8400-e29b-41d4-a716-446655440000",
        "--json",
    ])
    .expect("expected inspect command to parse");

    match cli.command {
        TickCommand::Inspect { job_id } => {
            assert_eq!(job_id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
            assert!(cli.json);
        }
        other => panic!("expected inspect command, got {other:?}"),
    }
}
