use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::ScheduledJob;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TickEvent {
    pub rai_component: String,
    pub rai_level: String,
    pub event: String,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cron_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    pub message: String,
}

impl TickEvent {
    #[must_use]
    pub fn for_job(
        level: &str,
        event: &str,
        timestamp: DateTime<Utc>,
        job: &ScheduledJob,
        selected_agent_id: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rai_component: "tick".to_owned(),
            rai_level: level.to_owned(),
            event: event.to_owned(),
            timestamp,
            job_id: Some(job.id),
            cron_expression: Some(job.cron.clone()),
            agent_role: Some(job.role.clone()),
            selected_agent_id,
            command: Some(job.cmd.clone()),
            message: message.into(),
        }
    }

    #[must_use]
    pub fn daemon(level: &str, event: &str, message: impl Into<String>) -> Self {
        Self {
            rai_component: "tick".to_owned(),
            rai_level: level.to_owned(),
            event: event.to_owned(),
            timestamp: Utc::now(),
            job_id: None,
            cron_expression: None,
            agent_role: None,
            selected_agent_id: None,
            command: None,
            message: message.into(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TickResponse {
    JobAdded { job: ScheduledJob },
    Error { error: String },
}

impl TickResponse {
    pub fn print_json(&self) -> anyhow::Result<()> {
        println!("{}", serde_json::to_string(self)?);
        Ok(())
    }
}
