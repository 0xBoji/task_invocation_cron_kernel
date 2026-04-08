use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "tick")]
#[command(bin_name = "tick")]
#[command(about = "Mesh-aware distributed cron scheduler for the RAI ecosystem")]
pub struct TickCli {
    #[command(subcommand)]
    pub command: TickCommand,

    /// Emit JSON responses for one-shot commands.
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Subcommand, Debug)]
pub enum TickCommand {
    /// Start the background scheduler daemon.
    Daemon {
        /// How often the daemon rescans persisted jobs for newly added entries.
        #[arg(long, default_value_t = 2_000)]
        sync_interval_ms: u64,
    },
    /// Add a new cron job to the persisted scheduler config.
    Add {
        /// Cron expression in tokio-cron-scheduler format.
        #[arg(long)]
        cron: String,
        /// Agent role that must be idle before dispatching.
        #[arg(long)]
        role: String,
        /// Command payload to dispatch when the trigger fires.
        #[arg(long)]
        cmd: String,
    },
}
