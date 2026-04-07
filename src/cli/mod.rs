use clap::{Parser, Subcommand};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(name = "tick")]
#[command(bin_name = "tick")]
#[command(about = "RAI Pillar 10: Task Invocation Cron Kernel", long_about = None)]
pub struct TickCli {
    #[command(subcommand)]
    pub command: TickCommand,

    /// Global flag to emit JSON only.
    #[arg(short, long, global = true)]
    pub json: bool,
}

#[derive(Subcommand, Debug)]
pub enum TickCommand {
    /// Starts the orchestration daemon. 🛰️
    Start {
        /// Interval to poll the mesh (seconds).
        #[arg(short, long, default_value = "5")]
        interval: u64,
    },

    /// Schedules a new task. 📅
    Schedule {
        /// Task name.
        #[arg(short, long)]
        name: String,

        /// Command to execute (e.g. 'wasp run task.wasm').
        #[arg(short, long)]
        command: String,

        /// Optional cron schedule (e.g. '*/5 * * * *').
        #[arg(short, long)]
        schedule: Option<String>,
    },

    /// Lists all tasks in the queue. 📋
    List {
        /// Show only tasks with the given status.
        #[arg(short, long)]
        status: Option<String>,
    },

    /// Cancels a pending or running task. 🚫
    Cancel {
        /// The UUID of the task to cancel.
        id: Uuid,
    },

    /// Clears all completed or failed tasks. 🧹
    Clean,
}
