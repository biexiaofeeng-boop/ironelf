-- Minimal durable control-plane ownership for chimera handoff.

CREATE TABLE control_tasks (
    receipt_id TEXT PRIMARY KEY,
    intent_id TEXT NOT NULL UNIQUE,
    task_id TEXT NOT NULL UNIQUE,
    root_task_id TEXT NOT NULL,
    source_system TEXT NOT NULL,
    channel_id TEXT,
    external_thread_id TEXT,
    user_id TEXT NOT NULL,
    project_id TEXT,
    session_id TEXT,
    input_text TEXT NOT NULL,
    attachments_ref_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    interaction_summary TEXT,
    requested_goal TEXT NOT NULL,
    requested_constraints_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    priority_hint TEXT,
    risk_hint TEXT,
    requested_mode TEXT,
    observed_at_utc TIMESTAMPTZ NOT NULL,
    observed_at_local TEXT,
    timezone TEXT NOT NULL,
    node_id TEXT,
    status TEXT NOT NULL,
    accepted_by TEXT NOT NULL,
    accepted_at_utc TIMESTAMPTZ NOT NULL,
    queue_state TEXT NOT NULL,
    next_action TEXT NOT NULL,
    summary TEXT NOT NULL,
    intent_payload_json JSONB NOT NULL,
    intent_payload_hash TEXT NOT NULL,
    dispatch_request_json JSONB NOT NULL,
    latest_execution_result_json JSONB
);

CREATE INDEX idx_control_tasks_user ON control_tasks(user_id);
CREATE INDEX idx_control_tasks_status ON control_tasks(status);
CREATE INDEX idx_control_tasks_queue_state ON control_tasks(queue_state);
CREATE INDEX idx_control_tasks_accepted_at ON control_tasks(accepted_at_utc DESC);

CREATE TABLE control_task_events (
    event_id TEXT PRIMARY KEY,
    parent_event_id TEXT REFERENCES control_task_events(event_id) ON DELETE SET NULL,
    task_id TEXT NOT NULL REFERENCES control_tasks(task_id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    producer_type TEXT NOT NULL,
    producer_id TEXT NOT NULL,
    seq BIGINT NOT NULL,
    attempt INTEGER NOT NULL DEFAULT 1,
    causation_id TEXT,
    correlation_id TEXT,
    observed_at_utc TIMESTAMPTZ NOT NULL,
    observed_at_local TEXT,
    timezone TEXT NOT NULL,
    node_id TEXT,
    payload_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    UNIQUE (task_id, seq)
);

CREATE INDEX idx_control_task_events_task_seq ON control_task_events(task_id, seq);
CREATE INDEX idx_control_task_events_correlation ON control_task_events(correlation_id);
