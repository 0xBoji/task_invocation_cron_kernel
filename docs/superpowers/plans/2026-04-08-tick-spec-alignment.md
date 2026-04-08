# TICK Spec Alignment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align `tick` with `docs/spec.md`, add repo-local `AGENTS.md`, and leave the crate building and verified.

**Architecture:** Replace the ad-hoc queue poller with a spec-shaped scheduler service built around persisted jobs, cron validation, mesh peer filtering, and strict JSON event emission. Keep the first version thin: persist jobs locally, discover idle peers through CAMP JSON, and simulate dispatch while keeping the boundaries ready for later `wasp` execution.

**Tech Stack:** Rust, clap, tokio, tokio-cron-scheduler, serde, serde_json, anyhow

---

### Task 1: Lock the CLI/output contract with failing tests

**Files:**
- Create: `tests/cli_contract.rs`
- Modify: `Cargo.toml`

- [ ] Step 1: Add CLI-focused test dependencies and a failing test for `tick add`/`tick daemon`
- [ ] Step 2: Run `cargo test cli_contract -- --nocapture` and confirm failures against the old CLI
- [ ] Step 3: Implement the minimum CLI changes to satisfy the tests
- [ ] Step 4: Re-run the focused tests until green

### Task 2: Lock engine behavior with failing tests

**Files:**
- Create: `tests/engine_behavior.rs`
- Modify: `src/engine/*`, `src/models/*`, `src/output.rs`

- [ ] Step 1: Add failing tests for cron validation, idle-agent selection, skip logging, and dispatch logging
- [ ] Step 2: Run the focused engine tests and confirm red
- [ ] Step 3: Implement the smallest engine/storage/logging changes that make them pass
- [ ] Step 4: Re-run focused tests until green

### Task 3: Add repo-local guidance

**Files:**
- Create: `AGENTS.md`

- [ ] Step 1: Model the file on sibling RAI repos while making `docs/spec.md` authoritative for TICK
- [ ] Step 2: Include product contract, expected module boundaries, verification commands, and lore commit guidance

### Task 4: Full verification

**Files:**
- Modify: as needed from previous tasks

- [ ] Step 1: Run `cargo fmt --all`
- [ ] Step 2: Run `cargo test`
- [ ] Step 3: Run `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Step 4: Run `cargo build`
- [ ] Step 5: Fix any remaining issues and re-run the full verification set
