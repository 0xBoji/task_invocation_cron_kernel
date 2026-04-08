use task_invocation_cron_kernel::models::{ExecutionPolicy, JobType, ScheduledJob, WasmJob};
use task_invocation_cron_kernel::storage::JobStore;
use uuid::Uuid;

fn sample_job() -> ScheduledJob {
    ScheduledJob::new(
        "*/5 * * * * *".to_owned(),
        "coder".to_owned(),
        ExecutionPolicy::MeshAny,
        JobType::Wasm(WasmJob {
            module: "./jobs/task.wasm".to_owned(),
            args: Vec::new(),
            allow_dirs: vec!["./workspace".to_owned()],
            env: Vec::new(),
        }),
    )
}

#[test]
fn store_can_find_job_by_id() {
    let root = std::env::temp_dir().join(format!("tick-store-inspect-{}", Uuid::new_v4()));
    let store = JobStore::new_in(&root).expect("store should initialize");
    let job = store.add_job(sample_job()).expect("job should persist");

    let found = store
        .find_job(job.id)
        .expect("lookup should succeed")
        .expect("job should exist");
    assert_eq!(found.id, job.id);
}

#[test]
fn store_returns_none_for_unknown_job() {
    let root = std::env::temp_dir().join(format!("tick-store-missing-{}", Uuid::new_v4()));
    let store = JobStore::new_in(&root).expect("store should initialize");

    let found = store
        .find_job(Uuid::new_v4())
        .expect("lookup should succeed");
    assert!(found.is_none());
}
