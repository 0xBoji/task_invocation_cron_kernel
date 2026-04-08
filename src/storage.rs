use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use directories::ProjectDirs;

use crate::models::ScheduledJob;

#[derive(Debug, Clone)]
pub struct JobStore {
    jobs_file: PathBuf,
}

impl JobStore {
    pub fn from_env_or_default() -> Result<Self> {
        if let Some(dir) = std::env::var_os("TICK_DATA_DIR") {
            return Self::new_in(PathBuf::from(dir));
        }

        let project_dirs = ProjectDirs::from("com", "rai", "tick")
            .context("failed to resolve tick data directory")?;

        Self::new_in(project_dirs.data_dir())
    }

    pub fn new_in(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        fs::create_dir_all(root)
            .with_context(|| format!("failed to create data dir `{}`", root.display()))?;

        let jobs_file = root.join("tick_jobs.json");
        if !jobs_file.exists() {
            fs::write(&jobs_file, "[]")
                .with_context(|| format!("failed to initialize `{}`", jobs_file.display()))?;
        }

        Ok(Self { jobs_file })
    }

    pub fn add_job(&self, job: ScheduledJob) -> Result<ScheduledJob> {
        let mut jobs = self.list_jobs()?;
        jobs.push(job.clone());
        self.write_jobs(&jobs)?;
        Ok(job)
    }

    pub fn list_jobs(&self) -> Result<Vec<ScheduledJob>> {
        let raw = fs::read_to_string(&self.jobs_file)
            .with_context(|| format!("failed to read `{}`", self.jobs_file.display()))?;

        if raw.trim().is_empty() {
            return Ok(Vec::new());
        }

        serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse `{}`", self.jobs_file.display()))
    }

    fn write_jobs(&self, jobs: &[ScheduledJob]) -> Result<()> {
        let payload = serde_json::to_string_pretty(jobs)?;
        fs::write(&self.jobs_file, payload)
            .with_context(|| format!("failed to write `{}`", self.jobs_file.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::JobStore;
    use crate::models::{ExecutionPolicy, JobType, ScheduledJob, WasmJob};
    use uuid::Uuid;

    #[test]
    fn add_job_persists_to_disk() {
        let root = std::env::temp_dir().join(format!("tick-store-test-{}", Uuid::new_v4()));
        let store = JobStore::new_in(&root).expect("store should initialize");

        let job = store
            .add_job(ScheduledJob::new(
                "*/5 * * * * *".to_owned(),
                "coder".to_owned(),
                ExecutionPolicy::MeshAny,
                JobType::Wasm(WasmJob {
                    module: "./jobs/task.wasm".to_owned(),
                    args: Vec::new(),
                    allow_dirs: Vec::new(),
                    env: Vec::new(),
                }),
            ))
            .expect("job should persist");

        let jobs = store.list_jobs().expect("jobs should load");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, job.id);
    }
}
