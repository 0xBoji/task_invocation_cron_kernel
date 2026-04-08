use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use tokio::time;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::{
    mesh::CampMesh,
    models::{MeshAgent, ScheduledJob},
    output::TickEvent,
    storage::JobStore,
};

pub use crate::models::validate_cron_expression;

pub trait EventSink: Send + Sync {
    fn emit(&self, event: &TickEvent) -> Result<()>;
}

#[derive(Debug, Default)]
pub struct StdoutEventSink;

impl EventSink for StdoutEventSink {
    fn emit(&self, event: &TickEvent) -> Result<()> {
        println!("{}", serde_json::to_string(event)?);
        Ok(())
    }
}

pub fn select_idle_agent<'a>(agents: &'a [MeshAgent], role: &str) -> Option<&'a MeshAgent> {
    agents
        .iter()
        .find(|agent| agent.role == role && agent.status.eq_ignore_ascii_case("idle"))
}

pub fn build_trigger_event(
    job: &ScheduledJob,
    agents: &[MeshAgent],
    at: chrono::DateTime<Utc>,
) -> TickEvent {
    match select_idle_agent(agents, &job.role) {
        Some(agent) => TickEvent::for_job(
            "info",
            "job_dispatched",
            at,
            job,
            Some(agent.id.clone()),
            format!(
                "dispatched `{}` to idle agent `{}` for role `{}`",
                job.cmd, agent.id, job.role
            ),
        ),
        None => TickEvent::for_job(
            "warn",
            "job_skipped",
            at,
            job,
            None,
            format!(
                "skipped trigger because no idle agent was available for role `{}`",
                job.role
            ),
        ),
    }
}

pub struct TickDaemon {
    store: Arc<Mutex<JobStore>>,
    mesh: CampMesh,
    sink: Arc<dyn EventSink>,
    sync_interval: Duration,
    scheduled_jobs: Arc<Mutex<HashSet<Uuid>>>,
}

impl TickDaemon {
    pub fn new(store: JobStore, sync_interval: Duration) -> Self {
        Self::with_sink(store, Arc::new(StdoutEventSink), sync_interval)
    }

    pub fn with_sink(store: JobStore, sink: Arc<dyn EventSink>, sync_interval: Duration) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            mesh: CampMesh::default(),
            sink,
            sync_interval,
            scheduled_jobs: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub async fn run(&self) -> Result<()> {
        let mut scheduler = JobScheduler::new().await?;
        self.sync_jobs(&scheduler).await?;
        self.sink.emit(&TickEvent::daemon(
            "info",
            "daemon_started",
            "tick daemon started and watching persisted jobs",
        ))?;
        scheduler.start().await?;

        let mut interval = time::interval(self.sync_interval);
        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => break,
                _ = interval.tick() => {
                    self.sync_jobs(&scheduler).await?;
                }
            }
        }

        scheduler.shutdown().await?;
        self.sink.emit(&TickEvent::daemon(
            "info",
            "daemon_stopped",
            "tick daemon stopped",
        ))?;
        Ok(())
    }

    async fn sync_jobs(&self, scheduler: &JobScheduler) -> Result<()> {
        let jobs = self
            .store
            .lock()
            .map_err(|_| anyhow!("job store mutex poisoned"))?
            .list_jobs()?;

        for job in jobs {
            let should_schedule = {
                let mut scheduled = self
                    .scheduled_jobs
                    .lock()
                    .map_err(|_| anyhow!("scheduled jobs mutex poisoned"))?;
                scheduled.insert(job.id)
            };

            if should_schedule {
                self.schedule_job(scheduler, job).await?;
            }
        }

        Ok(())
    }

    async fn schedule_job(&self, scheduler: &JobScheduler, job: ScheduledJob) -> Result<()> {
        let mesh = self.mesh.clone();
        let sink = Arc::clone(&self.sink);
        let cron = job.cron.clone();
        let job_for_schedule = job.clone();

        let scheduled_job = Job::new_async(&cron, move |_job_id, _scheduler| {
            let mesh = mesh.clone();
            let sink = Arc::clone(&sink);
            let job = job_for_schedule.clone();
            Box::pin(async move {
                let event = match mesh.fetch_agents().await {
                    Ok(agents) => build_trigger_event(&job, &agents, Utc::now()),
                    Err(error) => TickEvent::for_job(
                        "error",
                        "job_failed",
                        Utc::now(),
                        &job,
                        None,
                        format!("failed to query mesh before dispatch: {error}"),
                    ),
                };

                if let Err(error) = sink.emit(&event) {
                    eprintln!("tick: failed to emit event: {error}");
                }
            })
        })
        .with_context(|| format!("failed to schedule cron `{cron}` for job {}", job.id))?;

        scheduler
            .add(scheduled_job)
            .await
            .with_context(|| format!("failed to register scheduled job {}", job.id))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{build_trigger_event, select_idle_agent, validate_cron_expression};
    use crate::models::{MeshAgent, ScheduledJob};

    #[test]
    fn cron_validation_accepts_six_field_expression() {
        validate_cron_expression("*/5 * * * * *").expect("valid cron should pass");
    }

    #[test]
    fn select_idle_agent_returns_none_when_all_busy() {
        let agents = vec![MeshAgent::new("agent-1", "coder", "busy")];
        assert!(select_idle_agent(&agents, "coder").is_none());
    }

    #[test]
    fn build_trigger_event_reports_dispatch() {
        let job = ScheduledJob::new_for_test("*/5 * * * * *", "coder", "echo hi");
        let agents = vec![MeshAgent::new("agent-1", "coder", "idle")];

        let event = build_trigger_event(&job, &agents, chrono::Utc::now());
        assert_eq!(event.event, "job_dispatched");
        assert_eq!(event.selected_agent_id.as_deref(), Some("agent-1"));
    }
}
