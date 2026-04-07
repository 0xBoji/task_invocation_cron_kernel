use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub name: String,
    pub command: String,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub schedule: Option<String>, // Cron expression
    pub last_run: Option<DateTime<Utc>>,
    pub assigned_agent: Option<String>,
    pub result: Option<serde_json::Value>,
}

impl Task {
    pub fn new(name: String, command: String, schedule: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            command,
            status: TaskStatus::Pending,
            created_at: now,
            updated_at: now,
            schedule,
            last_run: None,
            assigned_agent: None,
            result: None,
        }
    }
}
