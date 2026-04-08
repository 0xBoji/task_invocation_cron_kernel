use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_cron_scheduler::Job;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduledJob {
    pub id: Uuid,
    pub cron: String,
    pub role: String,
    pub cmd: String,
    pub created_at: DateTime<Utc>,
}

impl ScheduledJob {
    #[must_use]
    pub fn new(cron: String, role: String, cmd: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            cron,
            role,
            cmd,
            created_at: Utc::now(),
        }
    }

    #[must_use]
    pub fn new_for_test(cron: &str, role: &str, cmd: &str) -> Self {
        Self::new(cron.to_owned(), role.to_owned(), cmd.to_owned())
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
