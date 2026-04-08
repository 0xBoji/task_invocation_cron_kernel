use std::sync::{Arc, Mutex};

use task_invocation_cron_kernel::engine::{Dispatcher, MeshProvider, SchedulerEngine};
use task_invocation_cron_kernel::models::{
    ExecutionPolicy, JobType, MeshAgent, ScheduledJob, WasmJob,
};

struct FakeMeshProvider {
    local_agent_id: Option<String>,
    agents: Vec<MeshAgent>,
}

impl MeshProvider for FakeMeshProvider {
    fn local_agent_id(&self) -> Option<&str> {
        self.local_agent_id.as_deref()
    }

    fn list_agents<'a>(
        &'a self,
    ) -> task_invocation_cron_kernel::engine::BoxFuture<'a, anyhow::Result<Vec<MeshAgent>>> {
        let agents = self.agents.clone();
        Box::pin(async move { Ok(agents) })
    }
}

#[derive(Default)]
struct FakeDispatcher {
    calls: Arc<Mutex<Vec<String>>>,
}

impl Dispatcher for FakeDispatcher {
    fn dispatch_local<'a>(
        &'a self,
        job: &'a ScheduledJob,
    ) -> task_invocation_cron_kernel::engine::BoxFuture<'a, anyhow::Result<String>> {
        let calls = Arc::clone(&self.calls);
        let preview = job.command_preview();
        Box::pin(async move {
            calls.lock().expect("mutex poisoned").push(preview.clone());
            Ok(preview)
        })
    }
}

fn wasm_job(policy: ExecutionPolicy) -> ScheduledJob {
    ScheduledJob::new(
        "*/5 * * * * *".to_owned(),
        "coder".to_owned(),
        policy,
        JobType::Wasm(WasmJob {
            module: "./jobs/task.wasm".to_owned(),
            args: Vec::new(),
            allow_dirs: vec!["./workspace".to_owned()],
            env: Vec::new(),
        }),
    )
}

#[tokio::test]
async fn mesh_any_round_robins_across_idle_candidates() {
    let engine = SchedulerEngine::new(
        FakeMeshProvider {
            local_agent_id: Some("local-agent".to_owned()),
            agents: vec![
                MeshAgent::new("agent-b", "coder", "idle"),
                MeshAgent::new("agent-a", "coder", "idle"),
            ],
        },
        FakeDispatcher::default(),
    );
    let job = wasm_job(ExecutionPolicy::MeshAny);

    let first = engine.evaluate_job(&job).await;
    let second = engine.evaluate_job(&job).await;

    assert_eq!(first.selected_agent_id.as_deref(), Some("agent-a"));
    assert_eq!(second.selected_agent_id.as_deref(), Some("agent-b"));
}

#[tokio::test]
async fn remote_only_simulates_remote_dispatch_without_local_execution() {
    let dispatcher = FakeDispatcher::default();
    let calls = Arc::clone(&dispatcher.calls);
    let engine = SchedulerEngine::new(
        FakeMeshProvider {
            local_agent_id: Some("local-agent".to_owned()),
            agents: vec![
                MeshAgent::new("local-agent", "coder", "idle"),
                MeshAgent::new("remote-agent", "coder", "idle"),
            ],
        },
        dispatcher,
    );

    let event = engine
        .evaluate_job(&wasm_job(ExecutionPolicy::RemoteOnly))
        .await;

    assert_eq!(event.event, "remote_dispatch_simulated");
    assert_eq!(event.rai_component, "tick");
    assert_eq!(event.rai_level, "info");
    assert_eq!(event.selected_agent_id.as_deref(), Some("remote-agent"));
    assert_eq!(calls.lock().expect("mutex poisoned").len(), 0);
}

#[tokio::test]
async fn local_only_dispatches_locally_and_marks_selected_agent_local() {
    let dispatcher = FakeDispatcher::default();
    let calls = Arc::clone(&dispatcher.calls);
    let engine = SchedulerEngine::new(
        FakeMeshProvider {
            local_agent_id: Some("local-agent".to_owned()),
            agents: vec![
                MeshAgent::new("remote-agent", "coder", "idle"),
                MeshAgent::new("local-agent", "coder", "idle"),
            ],
        },
        dispatcher,
    );

    let event = engine
        .evaluate_job(&wasm_job(ExecutionPolicy::LocalOnly))
        .await;

    assert_eq!(event.event, "job_dispatched");
    assert_eq!(event.selected_agent_id.as_deref(), Some("local-agent"));
    assert_eq!(event.selected_agent_local, Some(true));
    assert_eq!(calls.lock().expect("mutex poisoned").len(), 1);
}
