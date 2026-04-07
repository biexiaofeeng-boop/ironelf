use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::db::Database;

const CONTROL_PLANE_SCHEMA_VERSION: &str = "v1";
const CONTROL_PLANE_ACCEPTED_BY: &str = "chimera-iceclaw.control-plane";
const CONTROL_PLANE_STATUS_ACCEPTED: &str = "accepted";
const CONTROL_PLANE_QUEUE_ACCEPTED: &str = "accepted";
const CONTROL_PLANE_NEXT_ACTION_DISPATCH_PENDING: &str = "dispatch_pending";
const CONTROL_PLANE_TARGET_RUNTIME: &str = "runtime_bridge";
const CONTROL_PLANE_EVENT_ACCEPTED: &str = "task.accepted";
const CONTROL_PLANE_PRODUCER_TYPE: &str = "control_plane";

fn default_schema_version() -> String {
    CONTROL_PLANE_SCHEMA_VERSION.to_string()
}

fn empty_array() -> Value {
    Value::Array(Vec::new())
}

fn empty_object() -> Value {
    Value::Object(serde_json::Map::new())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlPlaneErrorDetail {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlPlaneErrorEnvelope {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    pub error: ControlPlaneErrorDetail,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ControlPlaneResponseError {
    http_status: u16,
    code: String,
    message: String,
    intent_id: Option<String>,
    task_id: Option<String>,
}

impl ControlPlaneResponseError {
    fn new(http_status: u16, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            http_status,
            code: code.into(),
            message: message.into(),
            intent_id: None,
            task_id: None,
        }
    }

    fn with_intent_id(mut self, intent_id: impl Into<String>) -> Self {
        self.intent_id = Some(intent_id.into());
        self
    }

    fn with_task_id(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    pub fn http_status(&self) -> u16 {
        self.http_status
    }

    pub fn to_envelope(&self) -> ControlPlaneErrorEnvelope {
        ControlPlaneErrorEnvelope {
            schema_version: default_schema_version(),
            error: ControlPlaneErrorDetail {
                code: self.code.clone(),
                message: self.message.clone(),
            },
            intent_id: self.intent_id.clone(),
            task_id: self.task_id.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskIntent {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    pub intent_id: String,
    pub source_system: String,
    #[serde(default)]
    pub channel_id: Option<String>,
    #[serde(default)]
    pub external_thread_id: Option<String>,
    pub user_id: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    pub input_text: String,
    #[serde(default = "empty_array")]
    pub attachments_ref: Value,
    #[serde(default)]
    pub interaction_summary: Option<String>,
    pub requested_goal: String,
    #[serde(default = "empty_object")]
    pub requested_constraints: Value,
    #[serde(default)]
    pub priority_hint: Option<String>,
    #[serde(default)]
    pub risk_hint: Option<String>,
    #[serde(default)]
    pub requested_mode: Option<String>,
    pub observed_at_utc: DateTime<Utc>,
    #[serde(default)]
    pub observed_at_local: Option<String>,
    pub timezone: String,
    #[serde(default)]
    pub node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskReceipt {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    pub receipt_id: String,
    pub intent_id: String,
    pub task_id: String,
    pub root_task_id: String,
    pub status: String,
    pub accepted_by: String,
    pub accepted_at_utc: DateTime<Utc>,
    pub queue_state: String,
    pub next_action: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchRequest {
    pub dispatch_id: String,
    pub task_id: String,
    #[serde(default)]
    pub assignment_id: Option<String>,
    pub target_runtime: String,
    #[serde(default)]
    pub target_role: Option<String>,
    pub goal: String,
    #[serde(default = "empty_object")]
    pub constraints: Value,
    #[serde(default = "empty_object")]
    pub context_ref: Value,
    #[serde(default = "empty_array")]
    pub artifact_inputs: Value,
    #[serde(default)]
    pub allowed_capabilities: Vec<String>,
    #[serde(default = "empty_object")]
    pub approval_context: Value,
    #[serde(default)]
    pub sandbox_profile: Option<String>,
    #[serde(default)]
    pub secret_scope_ref: Option<String>,
    #[serde(default)]
    pub expected_outputs: Vec<String>,
    #[serde(default)]
    pub deadline: Option<DateTime<Utc>>,
    pub attempt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub result_id: String,
    pub task_id: String,
    #[serde(default)]
    pub assignment_id: Option<String>,
    pub status: String,
    pub summary: String,
    #[serde(default = "empty_array")]
    pub artifact_refs: Value,
    #[serde(default = "empty_array")]
    pub evidence_refs: Value,
    #[serde(default)]
    pub suggested_next_actions: Vec<String>,
    #[serde(default)]
    pub completed_at_utc: Option<DateTime<Utc>>,
    pub producer_type: String,
    pub producer_id: String,
    pub attempt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlTaskRecord {
    pub receipt_id: String,
    pub intent_id: String,
    pub task_id: String,
    pub root_task_id: String,
    pub source_system: String,
    pub channel_id: Option<String>,
    pub external_thread_id: Option<String>,
    pub user_id: String,
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub input_text: String,
    pub attachments_ref: Value,
    pub interaction_summary: Option<String>,
    pub requested_goal: String,
    pub requested_constraints: Value,
    pub priority_hint: Option<String>,
    pub risk_hint: Option<String>,
    pub requested_mode: Option<String>,
    pub observed_at_utc: DateTime<Utc>,
    pub observed_at_local: Option<String>,
    pub timezone: String,
    pub node_id: Option<String>,
    pub status: String,
    pub accepted_by: String,
    pub accepted_at_utc: DateTime<Utc>,
    pub queue_state: String,
    pub next_action: String,
    pub summary: String,
    pub intent_payload: Value,
    pub intent_payload_hash: String,
    pub dispatch_request: DispatchRequest,
    pub latest_execution_result: Option<ExecutionResult>,
}

impl ControlTaskRecord {
    pub fn to_receipt(&self) -> TaskReceipt {
        TaskReceipt {
            schema_version: default_schema_version(),
            receipt_id: self.receipt_id.clone(),
            intent_id: self.intent_id.clone(),
            task_id: self.task_id.clone(),
            root_task_id: self.root_task_id.clone(),
            status: self.status.clone(),
            accepted_by: self.accepted_by.clone(),
            accepted_at_utc: self.accepted_at_utc,
            queue_state: self.queue_state.clone(),
            next_action: self.next_action.clone(),
            summary: self.summary.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEventRecord {
    pub event_id: String,
    pub parent_event_id: Option<String>,
    pub task_id: String,
    pub event_type: String,
    pub producer_type: String,
    pub producer_id: String,
    pub seq: i64,
    pub attempt: i32,
    pub causation_id: Option<String>,
    pub correlation_id: Option<String>,
    pub observed_at_utc: DateTime<Utc>,
    pub observed_at_local: Option<String>,
    pub timezone: String,
    pub node_id: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone)]
pub enum ControlTaskAcceptance {
    Inserted(ControlTaskRecord),
    Existing(ControlTaskRecord),
    Conflict(ControlTaskRecord),
}

#[derive(Debug, Default, Clone)]
pub struct ControlPlaneManager;

impl ControlPlaneManager {
    pub async fn accept_task_intent(
        &self,
        intent: TaskIntent,
        authenticated_user_id: &str,
        store: Option<Arc<dyn Database>>,
    ) -> Result<TaskReceipt, ControlPlaneResponseError> {
        validate_intent(&intent, authenticated_user_id)?;

        let Some(store) = store else {
            return Err(ControlPlaneResponseError::new(
                503,
                "control_store_unavailable",
                "Database store is required for control-plane acceptance",
            )
            .with_intent_id(intent.intent_id));
        };

        let intent_payload = canonical_json_value(serde_json::to_value(&intent).map_err(|e| {
            ControlPlaneResponseError::new(
                500,
                "control_intent_serialize_failed",
                format!("Failed to serialize task intent: {e}"),
            )
            .with_intent_id(intent.intent_id.clone())
        })?);
        let intent_payload_hash = hash_json_value(&intent_payload);
        let accepted_at_utc = truncate_to_millis(Utc::now());
        let task_id = Uuid::new_v4().to_string();
        let record = ControlTaskRecord {
            receipt_id: Uuid::new_v4().to_string(),
            intent_id: intent.intent_id.clone(),
            task_id: task_id.clone(),
            root_task_id: task_id.clone(),
            source_system: intent.source_system.clone(),
            channel_id: intent.channel_id.clone(),
            external_thread_id: intent.external_thread_id.clone(),
            user_id: intent.user_id.clone(),
            project_id: intent.project_id.clone(),
            session_id: intent.session_id.clone(),
            input_text: intent.input_text.clone(),
            attachments_ref: intent.attachments_ref.clone(),
            interaction_summary: intent.interaction_summary.clone(),
            requested_goal: intent.requested_goal.clone(),
            requested_constraints: intent.requested_constraints.clone(),
            priority_hint: intent.priority_hint.clone(),
            risk_hint: intent.risk_hint.clone(),
            requested_mode: intent.requested_mode.clone(),
            observed_at_utc: intent.observed_at_utc,
            observed_at_local: intent.observed_at_local.clone(),
            timezone: intent.timezone.clone(),
            node_id: intent.node_id.clone(),
            status: CONTROL_PLANE_STATUS_ACCEPTED.to_string(),
            accepted_by: CONTROL_PLANE_ACCEPTED_BY.to_string(),
            accepted_at_utc,
            queue_state: CONTROL_PLANE_QUEUE_ACCEPTED.to_string(),
            next_action: CONTROL_PLANE_NEXT_ACTION_DISPATCH_PENDING.to_string(),
            summary: build_summary(&intent.requested_goal),
            intent_payload,
            intent_payload_hash,
            dispatch_request: build_dispatch_request(&intent, &task_id),
            latest_execution_result: None,
        };
        let accepted_event = build_accepted_event(&record);

        match store
            .accept_control_task(&record, &accepted_event)
            .await
            .map_err(|e| {
                ControlPlaneResponseError::new(
                    500,
                    "control_store_write_failed",
                    format!("Failed to persist control task: {e}"),
                )
                .with_intent_id(intent.intent_id.clone())
            })? {
            ControlTaskAcceptance::Inserted(record) | ControlTaskAcceptance::Existing(record) => {
                Ok(record.to_receipt())
            }
            ControlTaskAcceptance::Conflict(record) => Err(ControlPlaneResponseError::new(
                409,
                "control_task_conflict",
                "Intent ID already exists with a different normalized payload",
            )
            .with_intent_id(record.intent_id)
            .with_task_id(record.task_id)),
        }
    }
}

fn truncate_to_millis(timestamp: DateTime<Utc>) -> DateTime<Utc> {
    DateTime::from_timestamp_millis(timestamp.timestamp_millis()).unwrap_or(timestamp)
}

fn validate_intent(
    intent: &TaskIntent,
    authenticated_user_id: &str,
) -> Result<(), ControlPlaneResponseError> {
    require_non_empty(&intent.intent_id, "intent_id")?;
    require_non_empty(&intent.source_system, "source_system")?;
    require_non_empty(&intent.user_id, "user_id")?;
    require_non_empty(&intent.input_text, "input_text")?;
    require_non_empty(&intent.requested_goal, "requested_goal")?;
    require_non_empty(&intent.timezone, "timezone")?;

    if intent.user_id != authenticated_user_id {
        return Err(ControlPlaneResponseError::new(
            403,
            "control_user_mismatch",
            "Authenticated user does not match TaskIntent.user_id",
        )
        .with_intent_id(intent.intent_id.clone()));
    }

    Ok(())
}

fn require_non_empty(value: &str, field: &str) -> Result<(), ControlPlaneResponseError> {
    if value.trim().is_empty() {
        return Err(ControlPlaneResponseError::new(
            400,
            "control_intent_invalid",
            format!("Field `{field}` must not be empty"),
        ));
    }
    Ok(())
}

fn build_summary(requested_goal: &str) -> String {
    let preview = truncate_chars(requested_goal.trim(), 96);
    format!("Accepted task intent: {preview}")
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut chars = input.chars();
    for _ in 0..max_chars {
        let Some(ch) = chars.next() else {
            return input.to_string();
        };
        out.push(ch);
    }
    if chars.next().is_some() {
        out.push_str("...");
    }
    out
}

fn build_dispatch_request(intent: &TaskIntent, task_id: &str) -> DispatchRequest {
    DispatchRequest {
        dispatch_id: Uuid::new_v4().to_string(),
        task_id: task_id.to_string(),
        assignment_id: None,
        target_runtime: CONTROL_PLANE_TARGET_RUNTIME.to_string(),
        target_role: intent.requested_mode.clone(),
        goal: intent.requested_goal.clone(),
        constraints: intent.requested_constraints.clone(),
        context_ref: serde_json::json!({
            "intent_id": intent.intent_id,
            "source_system": intent.source_system,
            "channel_id": intent.channel_id,
            "external_thread_id": intent.external_thread_id,
            "project_id": intent.project_id,
            "session_id": intent.session_id,
        }),
        artifact_inputs: intent.attachments_ref.clone(),
        allowed_capabilities: Vec::new(),
        approval_context: empty_object(),
        sandbox_profile: None,
        secret_scope_ref: None,
        expected_outputs: Vec::new(),
        deadline: None,
        attempt: 0,
    }
}

fn build_accepted_event(record: &ControlTaskRecord) -> TaskEventRecord {
    TaskEventRecord {
        event_id: Uuid::new_v4().to_string(),
        parent_event_id: None,
        task_id: record.task_id.clone(),
        event_type: CONTROL_PLANE_EVENT_ACCEPTED.to_string(),
        producer_type: CONTROL_PLANE_PRODUCER_TYPE.to_string(),
        producer_id: record.accepted_by.clone(),
        seq: 1,
        attempt: 1,
        causation_id: Some(record.intent_id.clone()),
        correlation_id: Some(record.intent_id.clone()),
        observed_at_utc: record.accepted_at_utc,
        observed_at_local: None,
        timezone: record.timezone.clone(),
        node_id: record.node_id.clone(),
        payload: serde_json::json!({
            "receipt_id": record.receipt_id,
            "status": record.status,
            "queue_state": record.queue_state,
            "next_action": record.next_action,
            "dispatch_id": record.dispatch_request.dispatch_id,
            "source_observed_at_utc": record.observed_at_utc,
            "source_observed_at_local": record.observed_at_local,
        }),
    }
}

fn canonical_json_value(value: Value) -> Value {
    match value {
        Value::Array(values) => {
            Value::Array(values.into_iter().map(canonical_json_value).collect())
        }
        Value::Object(map) => {
            let mut entries: Vec<_> = map.into_iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let mut sorted = serde_json::Map::with_capacity(entries.len());
            for (key, value) in entries {
                sorted.insert(key, canonical_json_value(value));
            }
            Value::Object(sorted)
        }
        other => other,
    }
}

fn hash_json_value(value: &Value) -> String {
    let bytes =
        serde_json::to_vec(value).expect("serializing canonical JSON value should not fail");
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::test_db;

    fn sample_intent(intent_id: &str, user_id: &str) -> TaskIntent {
        TaskIntent {
            schema_version: default_schema_version(),
            intent_id: intent_id.to_string(),
            source_system: "chimera-core".to_string(),
            channel_id: Some("gateway".to_string()),
            external_thread_id: Some("thread-123".to_string()),
            user_id: user_id.to_string(),
            project_id: Some("project-abc".to_string()),
            session_id: Some("session-xyz".to_string()),
            input_text: "Summarize the latest deployment health.".to_string(),
            attachments_ref: serde_json::json!([
                {"kind": "doc", "id": "artifact-1"}
            ]),
            interaction_summary: Some("handoff from control shell".to_string()),
            requested_goal: "Assess the deployment status and propose the next action.".to_string(),
            requested_constraints: serde_json::json!({
                "max_runtime_s": 90,
                "must_return_json": true
            }),
            priority_hint: Some("high".to_string()),
            risk_hint: Some("medium".to_string()),
            requested_mode: Some("worker".to_string()),
            observed_at_utc: DateTime::parse_from_rfc3339("2026-04-06T08:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            observed_at_local: Some("2026-04-06T16:00:00+08:00".to_string()),
            timezone: "Asia/Shanghai".to_string(),
            node_id: Some("chimera-core-node-a".to_string()),
        }
    }

    #[cfg(feature = "libsql")]
    #[tokio::test]
    async fn accept_task_intent_persists_receipt_dispatch_and_event() {
        let (db, _dir) = test_db().await;
        let manager = ControlPlaneManager;

        let receipt = manager
            .accept_task_intent(
                sample_intent("intent-accept-1", "alice"),
                "alice",
                Some(db.clone()),
            )
            .await
            .unwrap();

        assert_eq!(receipt.intent_id, "intent-accept-1");
        assert_eq!(receipt.status, CONTROL_PLANE_STATUS_ACCEPTED);
        assert_eq!(
            receipt.next_action,
            CONTROL_PLANE_NEXT_ACTION_DISPATCH_PENDING
        );

        let record = db
            .get_control_task_by_intent_id("intent-accept-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(record.task_id, receipt.task_id);
        assert_eq!(record.dispatch_request.task_id, receipt.task_id);
        assert_eq!(
            record.dispatch_request.target_runtime,
            CONTROL_PLANE_TARGET_RUNTIME
        );
        assert_eq!(
            record.observed_at_utc,
            sample_intent("intent-accept-1", "alice").observed_at_utc
        );
        assert_eq!(
            record.observed_at_local,
            Some("2026-04-06T16:00:00+08:00".to_string())
        );
        assert_eq!(record.timezone, "Asia/Shanghai");

        let events = db.list_control_task_events(&receipt.task_id).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, CONTROL_PLANE_EVENT_ACCEPTED);
        assert_eq!(events[0].seq, 1);
        assert_eq!(events[0].timezone, "Asia/Shanghai");
        assert_eq!(events[0].observed_at_utc, receipt.accepted_at_utc);
        assert_eq!(events[0].observed_at_local, None);
        assert_eq!(
            events[0].payload["source_observed_at_local"],
            "2026-04-06T16:00:00+08:00"
        );
    }

    #[cfg(feature = "libsql")]
    #[tokio::test]
    async fn duplicate_same_payload_returns_same_receipt_without_duplicate_event() {
        let (db, _dir) = test_db().await;
        let manager = ControlPlaneManager;
        let intent = sample_intent("intent-dedupe-1", "alice");

        let first = manager
            .accept_task_intent(intent.clone(), "alice", Some(db.clone()))
            .await
            .unwrap();
        let second = manager
            .accept_task_intent(intent, "alice", Some(db.clone()))
            .await
            .unwrap();

        assert_eq!(first, second);
        let events = db.list_control_task_events(&first.task_id).await.unwrap();
        assert_eq!(events.len(), 1);
    }

    #[cfg(feature = "libsql")]
    #[tokio::test]
    async fn duplicate_conflicting_payload_is_rejected_safely() {
        let (db, _dir) = test_db().await;
        let manager = ControlPlaneManager;
        let first = sample_intent("intent-conflict-1", "alice");
        manager
            .accept_task_intent(first, "alice", Some(db.clone()))
            .await
            .unwrap();

        let mut conflicting = sample_intent("intent-conflict-1", "alice");
        conflicting.input_text = "Conflicting payload".to_string();

        let err = manager
            .accept_task_intent(conflicting, "alice", Some(db))
            .await
            .unwrap_err();
        assert_eq!(err.http_status(), 409);
        assert_eq!(err.to_envelope().error.code, "control_task_conflict");
    }

    #[cfg(feature = "libsql")]
    #[tokio::test]
    async fn mismatched_user_is_rejected_before_writing() {
        let (db, _dir) = test_db().await;
        let manager = ControlPlaneManager;

        let err = manager
            .accept_task_intent(
                sample_intent("intent-user-mismatch", "alice"),
                "bob",
                Some(db.clone()),
            )
            .await
            .unwrap_err();
        assert_eq!(err.http_status(), 403);
        assert!(
            db.get_control_task_by_intent_id("intent-user-mismatch")
                .await
                .unwrap()
                .is_none()
        );
    }
}
