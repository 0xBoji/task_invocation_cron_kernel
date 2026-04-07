use serde::{Deserialize, Serialize};
use crate::models::Task;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum TickResponse {
    Success {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        task: Option<Task>,
    },
    Error {
        error: String,
    },
}

impl TickResponse {
    pub fn success(message: impl Into<String>, task: Option<Task>) -> Self {
        Self::Success {
            message: message.into(),
            task,
        }
    }

    pub fn error(err: impl Into<String>) -> Self {
        Self::Error {
            error: err.into(),
        }
    }

    pub fn print_json(&self) {
        if let Ok(json) = serde_json::to_string(self) {
            println!("{}", json);
        }
    }
}
