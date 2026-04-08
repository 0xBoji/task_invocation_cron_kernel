use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{ExecutionPolicy, Mode, ScheduledJob};

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
    pub execution_policy: Option<ExecutionPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_mode: Option<Mode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_agent_local: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_preview: Option<String>,
    pub message: String,
}

impl TickEvent {
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
            execution_policy: None,
            job_mode: None,
            selected_agent_id: None,
            selected_agent_local: None,
            command_preview: None,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn job_triggered(job: &ScheduledJob) -> Self {
        Self::for_job(
            "info",
            "job_triggered",
            job,
            None,
            None,
            None,
            "cron trigger fired",
        )
    }

    #[must_use]
    pub fn job_skipped(job: &ScheduledJob, message: impl Into<String>) -> Self {
        Self::for_job(
            "warn",
            "job_skipped",
            job,
            None,
            None,
            Some(job.command_preview()),
            message,
        )
    }

    #[must_use]
    pub fn remote_dispatch_simulated(
        job: &ScheduledJob,
        agent_id: String,
        message: impl Into<String>,
    ) -> Self {
        Self::for_job(
            "info",
            "remote_dispatch_simulated",
            job,
            Some(agent_id),
            Some(false),
            Some(job.command_preview()),
            message,
        )
    }

    #[must_use]
    pub fn job_dispatched(
        job: &ScheduledJob,
        agent_id: String,
        selected_agent_local: bool,
        command_preview: String,
        message: impl Into<String>,
    ) -> Self {
        Self::for_job(
            "info",
            "job_dispatched",
            job,
            Some(agent_id),
            Some(selected_agent_local),
            Some(command_preview),
            message,
        )
    }

    #[must_use]
    pub fn job_failed(
        job: &ScheduledJob,
        agent_id: Option<String>,
        selected_agent_local: Option<bool>,
        command_preview: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::for_job(
            "error",
            "job_failed",
            job,
            agent_id,
            selected_agent_local,
            command_preview,
            message,
        )
    }

    fn for_job(
        level: &str,
        event: &str,
        job: &ScheduledJob,
        selected_agent_id: Option<String>,
        selected_agent_local: Option<bool>,
        command_preview: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rai_component: "tick".to_owned(),
            rai_level: level.to_owned(),
            event: event.to_owned(),
            timestamp: Utc::now(),
            job_id: Some(job.id),
            cron_expression: Some(job.cron.clone()),
            agent_role: Some(job.role.clone()),
            execution_policy: Some(job.policy),
            job_mode: Some(job.mode()),
            selected_agent_id,
            selected_agent_local,
            command_preview,
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
