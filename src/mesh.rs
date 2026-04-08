use anyhow::{bail, Context, Result};
use tokio::process::Command;

use crate::models::MeshAgent;

#[derive(Debug, Clone)]
pub struct CampMesh {
    program: String,
}

impl Default for CampMesh {
    fn default() -> Self {
        Self {
            program: std::env::var("TICK_CAMP_BIN").unwrap_or_else(|_| "camp".to_owned()),
        }
    }
}

impl CampMesh {
    pub async fn fetch_agents(&self) -> Result<Vec<MeshAgent>> {
        let output = Command::new(&self.program)
            .args(["list", "--json"])
            .output()
            .await
            .with_context(|| format!("failed to execute `{}`", self.program))?;

        if !output.status.success() {
            bail!(
                "`{} list --json` exited with status {}",
                self.program,
                output.status
            );
        }

        serde_json::from_slice(&output.stdout)
            .context("failed to parse `camp list --json` output into mesh agents")
    }
}
