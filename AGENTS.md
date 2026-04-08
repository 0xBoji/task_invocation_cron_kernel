# AGENTS.md — task_invocation_cron_kernel

This file governs the entire `task_invocation_cron_kernel/` repository.

## Mission
Build `tick` (Task Invocation Cron Kernel), a production-quality Rust CLI that schedules cron jobs, queries the CAMP mesh for idle agents by role, and emits strict JSON daemon events for the RAI ecosystem.

The source of truth is `docs/spec.md`.
If implementation details are unclear, prefer the spec over assumptions.

## Product contract
- Binary name: `tick`
- Primary commands:
  - `tick daemon`
  - `tick add --cron "<expr>" --role "<agent_role>" --cmd "<command>"`
- Main behavior:
  - Persist scheduled jobs locally so `tick add` can feed a long-running daemon.
  - Use `tokio-cron-scheduler` to trigger stored jobs.
  - On each trigger, query the CAMP mesh and pick only agents whose `role` matches and whose `status` is `idle`.
  - If an idle agent exists, simulate dispatch and log the selected agent plus command.
  - If no idle agent exists, skip the tick and wait for the next cron trigger.
  - Daemon actions must emit strict JSON logs with RAI metadata.

## Required technical choices
Use these unless a stronger repo-local reason appears during implementation:
- Rust stable
- `clap` v4 with derive
- `tokio`
- `tokio-cron-scheduler`
- `serde` + `serde_json`
- `anyhow` for application flow
- CAMP integration should stay compatible with `coding_agent_mesh_presence`

## Expected module layout
Keep code small, explicit, and easy to test. Prefer this structure unless a better split emerges naturally:
- `src/main.rs` — CLI entrypoint and top-level command routing
- `src/cli/` — clap command/flag definitions
- `src/models/` — typed job and mesh-agent data models
- `src/storage.rs` — persisted job config loading/writing
- `src/mesh.rs` — CAMP querying boundary
- `src/engine/` — cron scheduler orchestration and trigger decisions
- `src/output.rs` — JSON event/response types
- `tests/` — CLI and engine contract tests

Do not add abstraction layers with no clear payoff.
Prefer deletion and consolidation over speculative architecture.

## Output contract
`tick daemon` is an Agent-Computer Interface for the wider RAI stack.
- Stdout daemon logs must be valid JSON.
- Every daemon event must include:
  - `rai_component = "tick"`
  - `rai_level`
  - `timestamp`
  - job context (`job_id`, `cron_expression`, `agent_role`, `command`) when applicable
  - `selected_agent_id` when a dispatch target exists
- Human-oriented text must not pollute daemon stdout.

## Code quality rules
- No `unwrap()` or `expect()` in production paths.
- Keep scheduling logic and JSON formatting separate.
- Make mesh-selection decisions explicit and typed.
- Add comments only where they preserve non-obvious operational behavior.
- No new dependencies beyond the spec unless clearly justified.

## Testing and verification
Before claiming work complete, run the smallest relevant full set:
- `cargo fmt --all`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build`

Also verify behavior relevant to the changed area, for example:
- CLI parsing for `tick daemon` and `tick add`
- cron expression validation
- idle-agent selection by role/status
- JSON event shape for dispatch vs skip

## Scope discipline
This repo is for `tick` only.
Do not add central queue servers, unrelated dashboards, or direct remote execution protocols. Keep dispatch simulation thin until the spec expands into real `wasp`/`wire` integration.

## Commit and agent-knowledge rules
- Treat git history as part of the agent memory for this repo.
- Every meaningful change should be committed with a Conventional Commit style subject:
  - `feat: ...`
  - `fix: ...`
  - `refactor: ...`
  - `test: ...`
  - `docs: ...`
  - `ci: ...`
  - `chore: ...`
- Prefer an optional scope when it improves clarity.
- The first line should explain why the change exists, not just what changed.
- For non-trivial commits, include brief lore-style trailers so future agents can recover intent quickly:
  - `Constraint:`
  - `Rejected:`
  - `Confidence:`
  - `Scope-risk:`
  - `Directive:`
  - `Tested:`
  - `Not-tested:`
- Do not batch unrelated changes into one commit.

## Completion checklist
A task is not done until all of the following are true:
- implementation still matches `docs/spec.md`
- code compiles
- relevant tests pass
- fmt + clippy pass cleanly
- daemon JSON output paths are verified
- final summary lists changed files, key decisions, and remaining risks
