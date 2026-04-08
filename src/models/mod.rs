use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use tokio_cron_scheduler::Job;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "kebab-case")]
pub enum ExecutionPolicy {
    LocalOnly,
    MeshAny,
    RemoteOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "kebab-case")]
pub enum Mode {
    Wasm,
    Shell,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
}

impl EnvVar {
    pub fn parse(raw: String) -> Result<Self> {
        let (key, value) = raw
            .split_once('=')
            .with_context(|| format!("invalid env var `{raw}`; expected KEY=VALUE"))?;

        if key.is_empty() {
            anyhow::bail!("invalid env var `{raw}`; key cannot be empty");
        }

        Ok(Self {
            key: key.to_owned(),
            value: value.to_owned(),
        })
    }

    #[must_use]
    pub fn as_cli_pair(&self) -> String {
        format!("{}={}", self.key, self.value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WasmJob {
    pub module: String,
    pub args: Vec<String>,
    pub allow_dirs: Vec<String>,
    pub env: Vec<EnvVar>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShellJob {
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum JobType {
    Wasm(WasmJob),
    Shell(ShellJob),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduledJob {
    pub id: Uuid,
    pub cron: String,
    pub role: String,
    pub policy: ExecutionPolicy,
    pub job_type: JobType,
    pub created_at: DateTime<Utc>,
}

impl ScheduledJob {
    #[must_use]
    pub fn new(cron: String, role: String, policy: ExecutionPolicy, job_type: JobType) -> Self {
        Self {
            id: Uuid::new_v4(),
            cron,
            role,
            policy,
            job_type,
            created_at: Utc::now(),
        }
    }

    #[must_use]
    pub const fn mode(&self) -> Mode {
        match self.job_type {
            JobType::Wasm(_) => Mode::Wasm,
            JobType::Shell(_) => Mode::Shell,
        }
    }

    #[must_use]
    pub fn command_preview(&self) -> String {
        match &self.job_type {
            JobType::Wasm(job) => {
                let mut parts = vec!["wasp".to_owned(), "run".to_owned(), quote(&job.module)];
                for allow_dir in &job.allow_dirs {
                    parts.push("--allow-dir".to_owned());
                    parts.push(quote(allow_dir));
                }
                for env in &job.env {
                    parts.push("--env".to_owned());
                    parts.push(quote(&env.as_cli_pair()));
                }
                if !job.args.is_empty() {
                    parts.push("# guest-args".to_owned());
                    parts.extend(job.args.iter().map(|arg| quote(arg)));
                }
                parts.join(" ")
            }
            JobType::Shell(job) => {
                let mut parts = vec![quote(&job.command)];
                parts.extend(job.args.iter().map(|arg| quote(arg)));
                parts.join(" ")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MeshAgent {
    pub id: String,
    pub role: String,
    pub status: String,
}

impl MeshAgent {
    #[must_use]
    pub fn new(id: &str, role: &str, status: &str) -> Self {
        Self {
            id: id.to_owned(),
            role: role.to_owned(),
            status: status.to_owned(),
        }
    }
}

pub fn validate_cron_expression(expression: &str) -> Result<()> {
    Job::new_async(expression, |_job_id, _scheduler| Box::pin(async {}))
        .map(|_| ())
        .with_context(|| format!("invalid cron expression `{expression}`"))
}

fn quote(value: &str) -> String {
    if value.contains(char::is_whitespace) {
        format!("\"{value}\"")
    } else {
        value.to_owned()
    }
}
