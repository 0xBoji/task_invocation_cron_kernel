use chrono::Utc;
use task_invocation_cron_kernel::engine::{
    build_trigger_event, select_idle_agent, validate_cron_expression,
};
use task_invocation_cron_kernel::models::{MeshAgent, ScheduledJob};

#[test]
fn cron_validation_rejects_invalid_expression() {
    let result = validate_cron_expression("not-a-cron");
    assert!(result.is_err(), "expected invalid cron expression to fail");
}

#[test]
fn idle_agent_selection_requires_matching_role() {
    let agents = vec![
        MeshAgent::new("agent-busy", "coder", "busy"),
        MeshAgent::new("agent-idle", "reviewer", "idle"),
        MeshAgent::new("agent-match", "coder", "idle"),
    ];

    let selected = select_idle_agent(&agents, "coder").expect("expected idle coder");
    assert_eq!(selected.id, "agent-match");
}

#[test]
fn skipped_trigger_event_is_warn_level() {
    let job = ScheduledJob::new_for_test("*/5 * * * * *", "coder", "echo hi");
    let event = build_trigger_event(&job, &[], Utc::now());

    assert_eq!(event.rai_level, "warn");
    assert_eq!(event.event, "job_skipped");
    assert_eq!(event.selected_agent_id, None);
}

#[test]
fn dispatched_trigger_event_includes_selected_agent() {
    let job = ScheduledJob::new_for_test("*/5 * * * * *", "coder", "echo hi");
    let agents = vec![MeshAgent::new("agent-007", "coder", "idle")];
    let event = build_trigger_event(&job, &agents, Utc::now());

    assert_eq!(event.rai_level, "info");
    assert_eq!(event.event, "job_dispatched");
    assert_eq!(event.selected_agent_id.as_deref(), Some("agent-007"));
}
