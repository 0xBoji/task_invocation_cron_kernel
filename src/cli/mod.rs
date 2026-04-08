use anyhow::{bail, Result};
use clap::{Args, Parser, Subcommand};

pub use crate::models::Mode;
use crate::models::{
    validate_cron_expression, EnvVar, ExecutionPolicy, JobType, ScheduledJob, ShellJob, WasmJob,
};

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
    Add(AddCommand),
    /// List all persisted jobs.
    List,
    /// Inspect a specific persisted job.
    Inspect {
        /// Job identifier to inspect.
        job_id: uuid::Uuid,
    },
}

#[derive(Args, Debug, Clone)]
pub struct AddCommand {
    /// Cron expression in tokio-cron-scheduler format.
    #[arg(long)]
    pub cron: String,
    /// Agent role that must be idle before dispatching.
    #[arg(long)]
    pub role: String,
    /// Execution policy deciding whether local and/or remote peers are eligible.
    #[arg(long, value_enum, default_value_t = ExecutionPolicy::MeshAny)]
    pub policy: ExecutionPolicy,
    /// Job payload mode. Wasm is the default.
    #[arg(long, value_enum, default_value_t = Mode::Wasm)]
    pub mode: Mode,
    /// Wasm module path for `--mode wasm`.
    #[arg(long)]
    pub module: Option<String>,
    /// Shell command for `--mode shell`.
    #[arg(long)]
    pub command: Option<String>,
    /// Arguments passed to the wasm task metadata or shell command.
    #[arg(long = "arg")]
    pub args: Vec<String>,
    /// Directories exposed to `wasp run`.
    #[arg(long = "allow-dir")]
    pub allow_dirs: Vec<String>,
    /// Environment variables for wasm execution in KEY=VALUE form.
    #[arg(long = "env")]
    pub env: Vec<String>,
}

impl AddCommand {
    pub fn into_job(self) -> Result<ScheduledJob> {
        validate_cron_expression(&self.cron)?;

        let job_type = match self.mode {
            Mode::Wasm => {
                if self.command.is_some() {
                    bail!("--command is only valid with --mode shell");
                }

                let module = self
                    .module
                    .ok_or_else(|| anyhow::anyhow!("--module is required with --mode wasm"))?;
                let env = self
                    .env
                    .into_iter()
                    .map(EnvVar::parse)
                    .collect::<Result<Vec<_>>>()?;

                JobType::Wasm(WasmJob {
                    module,
                    args: self.args,
                    allow_dirs: self.allow_dirs,
                    env,
                })
            }
            Mode::Shell => {
                if self.module.is_some() {
                    bail!("--module is only valid with --mode wasm");
                }
                if !self.allow_dirs.is_empty() {
                    bail!("--allow-dir is only valid with --mode wasm");
                }
                if !self.env.is_empty() {
                    bail!("--env is only valid with --mode wasm");
                }

                let command = self
                    .command
                    .ok_or_else(|| anyhow::anyhow!("--command is required with --mode shell"))?;

                JobType::Shell(ShellJob {
                    command,
                    args: self.args,
                })
            }
        };

        Ok(ScheduledJob::new(
            self.cron,
            self.role,
            self.policy,
            job_type,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{AddCommand, Mode};
    use crate::models::ExecutionPolicy;

    #[test]
    fn rejects_wasm_mode_without_module() {
        let error = AddCommand {
            cron: "*/5 * * * * *".to_owned(),
            role: "coder".to_owned(),
            policy: ExecutionPolicy::MeshAny,
            mode: Mode::Wasm,
            module: None,
            command: None,
            args: Vec::new(),
            allow_dirs: Vec::new(),
            env: Vec::new(),
        }
        .into_job()
        .expect_err("missing module should fail");

        assert!(error.to_string().contains("--module is required"));
    }
}
