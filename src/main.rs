mod cli;
mod engine;
mod models;
mod output;

use clap::Parser;
use cli::{TickCli, TickCommand};
use engine::Orchestrator;
use output::TickResponse;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = TickCli::parse();
    let orchestrator = Orchestrator::new()?;

    match cli.command {
        TickCommand::Schedule { name, command, schedule } => {
            match orchestrator.schedule_task(name, command, schedule) {
                Ok(task) => {
                    if cli.json {
                        TickResponse::success("Task scheduled", Some(task)).print_json();
                    } else {
                        println!("📅 Task scheduled with ID: {}", task.id);
                    }
                }
                Err(e) => {
                    if cli.json {
                        TickResponse::error(e.to_string()).print_json();
                    } else {
                        eprintln!("❌ Error scheduling task: {}", e);
                    }
                }
            }
        }
        TickCommand::List { status } => {
            match orchestrator.list_tasks(status) {
                Ok(tasks) => {
                    if cli.json {
                        println!("{}", serde_json::to_string_pretty(&tasks)?);
                    } else {
                        println!("📋 Current Tasks:");
                        for t in tasks {
                            println!("[{:?}] {}: {} (Assigned: {:?})", t.status, t.id, t.name, t.assigned_agent);
                        }
                    }
                }
                Err(e) => {
                    if cli.json {
                        TickResponse::error(e.to_string()).print_json();
                    } else {
                        eprintln!("❌ Error listing tasks: {}", e);
                    }
                }
            }
        }
        TickCommand::Start { interval } => {
            orchestrator.run_loop(interval).await?;
        }
        _ => {
            println!("Command logic for 'cancel' and 'clean' coming in Phase 2.");
        }
    }

    Ok(())
}
