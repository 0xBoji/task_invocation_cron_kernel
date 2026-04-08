use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use task_invocation_cron_kernel::{
    cli::{TickCli, TickCommand},
    engine::{LocalDispatcher, SchedulerEngine, TickDaemon},
    mesh::CampMeshProvider,
    output::TickResponse,
    storage::JobStore,
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = TickCli::parse();
    let store = JobStore::from_env_or_default()?;

    match cli.command {
        TickCommand::Add(add) => {
            let job = store.add_job(add.into_job()?)?;
            if cli.json {
                TickResponse::JobAdded { job }.print_json()?;
            } else {
                println!("added job {}", job.id);
            }
        }
        TickCommand::List => {
            let jobs = store.list_jobs()?;
            if cli.json {
                println!("{}", serde_json::to_string(&jobs)?);
            } else {
                for job in jobs {
                    println!(
                        "{} {:?} {:?} {}",
                        job.id,
                        job.policy,
                        job.mode(),
                        job.command_preview()
                    );
                }
            }
        }
        TickCommand::Inspect { job_id } => {
            let job = store.find_job(job_id)?;
            if cli.json {
                println!("{}", serde_json::to_string(&job)?);
            } else {
                match job {
                    Some(job) => println!(
                        "{} {:?} {:?} {}",
                        job.id,
                        job.policy,
                        job.mode(),
                        job.command_preview()
                    ),
                    None => println!("job {} not found", job_id),
                }
            }
        }
        TickCommand::Daemon { sync_interval_ms } => {
            let engine = SchedulerEngine::new(
                CampMeshProvider::from_env().await?,
                LocalDispatcher::default(),
            );
            let daemon = TickDaemon::new(store, engine, Duration::from_millis(sync_interval_ms));
            daemon.run().await?;
        }
    }

    Ok(())
}
