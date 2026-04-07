use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time;
use anyhow::{Context, Result};
use crate::models::{Task, TaskStatus};
use crate::output::TickResponse;
use directories::ProjectDirs;

pub struct Orchestrator {
    queue_dir: PathBuf,
}

impl Orchestrator {
    pub fn new() -> Result<Self> {
        let proj_dirs = ProjectDirs::from("com", "rai", "tick")
            .context("Could not determine project directories")?;
        let queue_dir = proj_dirs.data_dir().join("queue");
        
        fs::create_dir_all(&queue_dir).context("Failed to create queue directory")?;
        
        Ok(Self { queue_dir })
    }

    pub fn schedule_task(&self, name: String, command: String, schedule: Option<String>) -> Result<Task> {
        let task = Task::new(name, command, schedule);
        self.save_task(&task)?;
        Ok(task)
    }

    pub fn list_tasks(&self, status_filter: Option<String>) -> Result<Vec<Task>> {
        let mut tasks = Vec::new();
        for entry in fs::read_dir(&self.queue_dir)? {
            let entry = entry?;
            let content = fs::read_to_string(entry.path())?;
            let task: Task = serde_json::from_str(&content)?;
            
            if let Some(ref filter) = status_filter {
                if format!("{:?}", task.status).to_lowercase() != filter.to_lowercase() {
                    continue;
                }
            }
            tasks.push(task);
        }
        Ok(tasks)
    }

    pub fn save_task(&self, task: &Task) -> Result<()> {
        let file_path = self.queue_dir.join(format!("{}.json", task.id));
        let content = serde_json::to_string_pretty(task)?;
        fs::write(file_path, content)?;
        Ok(())
    }

    pub async fn run_loop(&self, interval_secs: u64) -> Result<()> {
        let mut interval = time::interval(Duration::from_secs(interval_secs));
        println!("🛰️ TICK Orchestrator started. Watching mesh every {}s...", interval_secs);

        loop {
            interval.tick().await;
            
            // 1. Identify pending tasks
            let mut tasks = self.list_tasks(Some("pending".into()))?;
            if tasks.is_empty() { continue; }

            // 2. Scan mesh for idle agents via 'camp list --json'
            let idle_agents = self.get_idle_agents().await?;
            if idle_agents.is_empty() {
                // No idle agents, wait for next tick
                continue;
            }

            // 3. Match and dispatch
            for agent_id in idle_agents {
                if let Some(mut task) = tasks.pop() {
                    println!("🚀 Dispatching task '{}' to agent '{}'", task.name, agent_id);
                    self.dispatch_task(&mut task, agent_id).await?;
                }
            }
        }
    }

    async fn get_idle_agents(&self) -> Result<Vec<String>> {
        // Attempt to call local 'camp list --json'
        let output = tokio::process::Command::new("camp")
            .arg("list")
            .arg("--json")
            .output()
            .await?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        // Assuming camp returns a list of AgentInfo objects
        let agents: Vec<serde_json::Value> = serde_json::from_str(&json_str).unwrap_or_default();
        
        let idle = agents.into_iter()
            .filter(|a| a["status"] == "idle" || a["status"] == "Idle")
            .filter_map(|a| a["id"].as_str().map(|s| s.to_string()))
            .collect();

        Ok(idle)
    }

    async fn dispatch_task(&self, task: &mut Task, agent_id: String) -> Result<()> {
        task.status = TaskStatus::Running;
        task.assigned_agent = Some(agent_id.clone());
        task.updated_at = chrono::Utc::now();
        self.save_task(task)?;

        // For v0.1.0, "dispatch" means updating CAMP status for the agent-id 
        // and executing the command locally as a worker proxy.
        // In a true distributed RAI, this would be a message over 'wire'.
        
        let cmd_parts: Vec<&str> = task.command.split_whitespace().collect();
        if cmd_parts.is_empty() { return Ok(()); }

        let mut child = tokio::process::Command::new(cmd_parts[0])
            .args(&cmd_parts[1..])
            .spawn()?;

        // We run it asynchronously but track the task
        let task_clone = task.clone();
        let queue_dir = self.queue_dir.clone();
        
        tokio::spawn(async move {
            let status = child.wait().await;
            let mut final_task = task_clone;
            final_task.status = if status.is_ok() && status.unwrap().success() {
                TaskStatus::Completed
            } else {
                TaskStatus::Failed
            };
            final_task.updated_at = chrono::Utc::now();
            let file_path = queue_dir.join(format!("{}.json", final_task.id));
            let _ = fs::write(file_path, serde_json::to_string_pretty(&final_task).unwrap());
            println!("✅ Task '{}' finished with status {:?}", final_task.name, final_task.status);
        });

        Ok(())
    }
}
