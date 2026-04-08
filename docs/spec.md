You are an Expert Rust Systems Engineer building production-grade infrastructure components. Your task is to create a complete, MVP-ready Rust crate for `task_invocation_cron_kernel` (CLI command: `tick`)—a mesh-aware distributed cron scheduler that integrates into a local Rust Agent Infrastructure (RAI) ecosystem.

## Context & Problem Statement

The user's multi-agent system currently responds only to manual prompts. They need a persistent daemon that can autonomously schedule and dispatch tasks to agents across a LAN, but with a critical constraint: **tasks are only dispatched to agents that are currently idle and have the required role**. The daemon must never overwhelm busy agents.

## The TICK Architecture

TICK operates as follows:
1. User schedules jobs via CLI with a cron expression, target agent role, and command to execute
2. When the cron trigger fires, the scheduler queries the `coding_agent_mesh_presence` (CAMP) mesh for agents matching that role with `status="idle"`
3. If an idle agent exists, dispatch the command to it
4. If no idle agent is available, skip the tick and retry on the next cron trigger
5. All actions emit structured JSON logs with RAI-specific metadata for downstream dashboard integration

## Requirements & Implementation Scope

**CLI Interface (`cli.rs`)**
- Command `tick daemon`: Starts the background scheduler engine
- Command `tick add --cron "<cron_expr>" --role "<agent_role>" --cmd "<command>"`: Adds a job to the daemon's memory/config
- Use `clap` with derive feature for CLI parsing

**Scheduler Engine (`engine.rs`)**
- Use `tokio-cron-scheduler` to trigger jobs based on cron expressions
- **Critical mesh logic:** When a cron trigger fires, query `coding_agent_mesh_presence` (assume this local crate exists and provides a method like `mesh.agents_by_role(role)`)
- Filter the results to find an agent where `status == "idle"`
- If found: Simulate dispatching the command and log the action (include agent ID and command)
- If not found: Skip the tick; do not retry or queue—wait for the next cron trigger
- Handle state safely using `Arc<Mutex<T>>` for long-running daemon stability; avoid panics

**Structured Event Logging**
- All daemon actions must emit strict JSON logs
- Every log entry MUST include:
  - `"rai_component": "tick"`
  - `"rai_level": "info"` (or `"warn"` if skipped, `"error"` if failed)
  - Additional context: job ID, cron expression, agent role, selected agent ID (if applicable), command, timestamp
- These logs enable the external VIEW dashboard to dynamically color-code and filter by component and severity

**Tech Stack (Non-negotiable)**
- Language: Rust
- CLI: `clap` (derive)
- Async Runtime: `tokio`
- Cron Scheduler: `tokio-cron-scheduler`
- Mesh Discovery: `coding_agent_mesh_presence` (local crate; assume it's available)
- Serialization: `serde` and `serde_json` for JSON output

## Deliverables

Provide the complete MVP crate structure:
- `Cargo.toml` with all dependencies and metadata
- `main.rs` with entry point and async runtime setup
- `cli.rs` with command definitions
- `engine.rs` with scheduler logic and mesh integration
- Optional `logger.rs` or inline logging functions

Ensure:
- Async code properly manages state without data races
- The daemon is stable enough for continuous operation (no unwrap panics; proper error handling)
- JSON output is valid and includes all required RAI metadata
- Code follows Rust idioms and best practices for long-running services

## Success Criteria

The user can immediately:
1. Run `cargo build` and `cargo run -- tick daemon` to start a long-running scheduler
2. Add jobs via `tick add` that execute based on cron timing
3. Observe JSON logs showing job triggers, agent selection, dispatch decisions, and skip events
4. Feed the JSON output directly to their external VIEW dashboard for visualization