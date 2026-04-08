use std::{
    collections::{HashMap, HashSet},
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use tokio::time;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::{
    models::{ExecutionPolicy, JobType, MeshAgent, ScheduledJob},
    output::TickEvent,
    storage::JobStore,
};

pub use crate::models::validate_cron_expression;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait MeshProvider: Send + Sync {
    fn local_agent_id(&self) -> Option<&str>;
    fn list_agents<'a>(&'a self) -> BoxFuture<'a, Result<Vec<MeshAgent>>>;
    fn shutdown<'a>(&'a self) -> BoxFuture<'a, Result<()>> {
        Box::pin(async { Ok(()) })
    }
}

pub trait Dispatcher: Send + Sync {
    fn dispatch_local<'a>(&'a self, job: &'a ScheduledJob) -> BoxFuture<'a, Result<String>>;
}

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

#[derive(Debug, Clone)]
pub struct LocalDispatcher {
    wasp_bin: String,
}

impl Default for LocalDispatcher {
    fn default() -> Self {
        Self {
            wasp_bin: std::env::var("TICK_WASP_BIN").unwrap_or_else(|_| "wasp".to_owned()),
        }
    }
}

impl Dispatcher for LocalDispatcher {
    fn dispatch_local<'a>(&'a self, job: &'a ScheduledJob) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            match &job.job_type {
                JobType::Wasm(wasm) => {
                    if !wasm.args.is_empty() {
                        anyhow::bail!(
                            "wasm guest args are not supported by `wasp run` yet; remove args or use --mode shell"
                        );
                    }

                    let mut command = tokio::process::Command::new(&self.wasp_bin);
                    command.arg("run").arg(&wasm.module);
                    for allow_dir in &wasm.allow_dirs {
                        command.arg("--allow-dir").arg(allow_dir);
                    }
                    for env in &wasm.env {
                        command.arg("--env").arg(env.as_cli_pair());
                    }

                    let output = command
                        .output()
                        .await
                        .with_context(|| format!("failed to spawn `{}`", self.wasp_bin))?;
                    let preview = job.command_preview();
                    if !output.status.success() {
                        anyhow::bail!(
                            "local wasp dispatch failed with status {}: {}",
                            output.status,
                            String::from_utf8_lossy(&output.stderr).trim()
                        );
                    }
                    Ok(preview)
                }
                JobType::Shell(shell) => {
                    let mut command = tokio::process::Command::new(&shell.command);
                    command.args(&shell.args);
                    let output = command
                        .output()
                        .await
                        .with_context(|| format!("failed to spawn `{}`", shell.command))?;
                    let preview = job.command_preview();
                    if !output.status.success() {
                        anyhow::bail!(
                            "local shell dispatch failed with status {}: {}",
                            output.status,
                            String::from_utf8_lossy(&output.stderr).trim()
                        );
                    }
                    Ok(preview)
                }
            }
        })
    }
}

pub struct SchedulerEngine<M, D> {
    mesh_provider: M,
    dispatcher: D,
    round_robin_state: Mutex<HashMap<Uuid, usize>>,
}

impl<M, D> SchedulerEngine<M, D>
where
    M: MeshProvider,
    D: Dispatcher,
{
    pub fn new(mesh_provider: M, dispatcher: D) -> Self {
        Self {
            mesh_provider,
            dispatcher,
            round_robin_state: Mutex::new(HashMap::new()),
        }
    }

    pub async fn evaluate_job(&self, job: &ScheduledJob) -> TickEvent {
        let agents = match self.mesh_provider.list_agents().await {
            Ok(agents) => agents,
            Err(error) => {
                return TickEvent::job_failed(
                    job,
                    None,
                    None,
                    Some(job.command_preview()),
                    format!("failed to query CAMP mesh: {error}"),
                );
            }
        };

        let local_agent_id = self.mesh_provider.local_agent_id().map(str::to_owned);
        let candidates = match eligible_candidates(job, agents, local_agent_id.as_deref()) {
            Ok(candidates) => candidates,
            Err(error) => {
                return TickEvent::job_failed(
                    job,
                    None,
                    None,
                    Some(job.command_preview()),
                    error.to_string(),
                );
            }
        };

        if candidates.is_empty() {
            return TickEvent::job_skipped(
                job,
                format!(
                    "skipped trigger because no idle agent matched role `{}` and policy `{:?}`",
                    job.role, job.policy
                ),
            );
        }

        let selected = select_round_robin_candidate(job.id, &candidates, &self.round_robin_state);
        let selected_is_local = local_agent_id
            .as_deref()
            .is_some_and(|agent_id| agent_id == selected.id);

        if selected_is_local {
            match self.dispatcher.dispatch_local(job).await {
                Ok(command_preview) => TickEvent::job_dispatched(
                    job,
                    selected.id.clone(),
                    true,
                    command_preview,
                    format!(
                        "dispatched job locally to idle agent `{}` using {:?}",
                        selected.id,
                        job.mode()
                    ),
                ),
                Err(error) => TickEvent::job_failed(
                    job,
                    Some(selected.id.clone()),
                    Some(true),
                    Some(job.command_preview()),
                    format!("local dispatch failed: {error}"),
                ),
            }
        } else {
            TickEvent::remote_dispatch_simulated(
                job,
                selected.id.clone(),
                format!("[TICK] Simulating remote dispatch to agent {}", selected.id),
            )
        }
    }

    pub fn local_agent_id(&self) -> Option<&str> {
        self.mesh_provider.local_agent_id()
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.mesh_provider.shutdown().await
    }
}

pub struct TickDaemon<M, D> {
    store: Arc<Mutex<JobStore>>,
    engine: Arc<SchedulerEngine<M, D>>,
    sink: Arc<dyn EventSink>,
    sync_interval: Duration,
    scheduled_jobs: Arc<Mutex<HashSet<Uuid>>>,
}

impl<M, D> TickDaemon<M, D>
where
    M: MeshProvider + 'static,
    D: Dispatcher + 'static,
{
    pub fn new(store: JobStore, engine: SchedulerEngine<M, D>, sync_interval: Duration) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            engine: Arc::new(engine),
            sink: Arc::new(StdoutEventSink),
            sync_interval,
            scheduled_jobs: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub async fn run(&self) -> Result<()> {
        let mut scheduler = JobScheduler::new().await?;
        self.sync_jobs(&scheduler).await?;
        self.emit(&TickEvent::daemon(
            "info",
            "daemon_started",
            "tick daemon started and watching persisted jobs",
        ));
        scheduler.start().await?;

        let mut interval = time::interval(self.sync_interval);
        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => break,
                _ = interval.tick() => {
                    if let Err(error) = self.sync_jobs(&scheduler).await {
                        self.emit(&TickEvent::daemon(
                            "error",
                            "job_sync_failed",
                            format!("failed to sync jobs: {error}"),
                        ));
                    }
                }
            }
        }

        scheduler.shutdown().await?;
        self.engine.shutdown().await?;
        self.emit(&TickEvent::daemon(
            "info",
            "daemon_stopped",
            "tick daemon stopped",
        ));
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
        let cron = job.cron.clone();
        let engine = Arc::clone(&self.engine);
        let sink = Arc::clone(&self.sink);
        let job_for_schedule = job.clone();

        let scheduled_job = Job::new_async(&cron, move |_job_id, _scheduler| {
            let engine = Arc::clone(&engine);
            let sink = Arc::clone(&sink);
            let job = job_for_schedule.clone();
            Box::pin(async move {
                if let Err(error) = sink.emit(&TickEvent::job_triggered(&job)) {
                    eprintln!("tick: failed to emit trigger event: {error}");
                }
                let event = engine.evaluate_job(&job).await;
                if let Err(error) = sink.emit(&event) {
                    eprintln!("tick: failed to emit job event: {error}");
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

    fn emit(&self, event: &TickEvent) {
        if let Err(error) = self.sink.emit(event) {
            eprintln!("tick: failed to emit daemon event: {error}");
        }
    }
}

fn eligible_candidates(
    job: &ScheduledJob,
    agents: Vec<MeshAgent>,
    local_agent_id: Option<&str>,
) -> Result<Vec<MeshAgent>> {
    let mut candidates = agents
        .into_iter()
        .filter(|agent| agent.role == job.role && agent.status.eq_ignore_ascii_case("idle"))
        .collect::<Vec<_>>();

    match job.policy {
        ExecutionPolicy::MeshAny => {}
        ExecutionPolicy::LocalOnly => {
            let local_agent_id = local_agent_id.ok_or_else(|| {
                anyhow!("cannot enforce local_only without a known local agent id")
            })?;
            candidates.retain(|agent| agent.id == local_agent_id);
        }
        ExecutionPolicy::RemoteOnly => {
            let local_agent_id = local_agent_id.ok_or_else(|| {
                anyhow!("cannot enforce remote_only without a known local agent id")
            })?;
            candidates.retain(|agent| agent.id != local_agent_id);
        }
    }

    candidates.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(candidates)
}

fn select_round_robin_candidate(
    job_id: Uuid,
    candidates: &[MeshAgent],
    state: &Mutex<HashMap<Uuid, usize>>,
) -> MeshAgent {
    let mut state = state.lock().expect("round-robin state mutex poisoned");
    let next_index = state.entry(job_id).or_insert(0);
    let selected = candidates[*next_index % candidates.len()].clone();
    *next_index = (*next_index + 1) % candidates.len();
    selected
}
