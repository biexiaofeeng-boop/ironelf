//! Control-plane persistence for the libSQL backend.

use async_trait::async_trait;
use libsql::params;

use super::{LibSqlBackend, fmt_ts, get_i64, get_json, get_opt_text, get_text, get_ts};
use crate::control_plane::{
    ControlTaskAcceptance, ControlTaskRecord, DispatchRequest, ExecutionResult, TaskEventRecord,
};
use crate::db::ControlPlaneStore;
use crate::error::DatabaseError;

fn encode_json(value: &serde_json::Value) -> Result<String, DatabaseError> {
    serde_json::to_string(value).map_err(|e| DatabaseError::Query(e.to_string()))
}

fn encode_dispatch_request(value: &DispatchRequest) -> Result<String, DatabaseError> {
    serde_json::to_string(value).map_err(|e| DatabaseError::Query(e.to_string()))
}

fn encode_execution_result(
    value: &Option<ExecutionResult>,
) -> Result<Option<String>, DatabaseError> {
    value
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| DatabaseError::Query(e.to_string()))
}

fn decode_control_task(row: &libsql::Row) -> Result<ControlTaskRecord, DatabaseError> {
    let dispatch_request: DispatchRequest =
        serde_json::from_value(get_json(row, 30)).map_err(|e| {
            DatabaseError::Query(format!(
                "Failed to decode control task dispatch_request_json: {e}"
            ))
        })?;
    let latest_execution_result = match get_opt_text(row, 31) {
        Some(raw) => Some(serde_json::from_str::<ExecutionResult>(&raw).map_err(|e| {
            DatabaseError::Query(format!(
                "Failed to decode control task latest_execution_result_json: {e}"
            ))
        })?),
        None => None,
    };

    Ok(ControlTaskRecord {
        receipt_id: get_text(row, 0),
        intent_id: get_text(row, 1),
        task_id: get_text(row, 2),
        root_task_id: get_text(row, 3),
        source_system: get_text(row, 4),
        channel_id: get_opt_text(row, 5),
        external_thread_id: get_opt_text(row, 6),
        user_id: get_text(row, 7),
        project_id: get_opt_text(row, 8),
        session_id: get_opt_text(row, 9),
        input_text: get_text(row, 10),
        attachments_ref: get_json(row, 11),
        interaction_summary: get_opt_text(row, 12),
        requested_goal: get_text(row, 13),
        requested_constraints: get_json(row, 14),
        priority_hint: get_opt_text(row, 15),
        risk_hint: get_opt_text(row, 16),
        requested_mode: get_opt_text(row, 17),
        observed_at_utc: get_ts(row, 18),
        observed_at_local: get_opt_text(row, 19),
        timezone: get_text(row, 20),
        node_id: get_opt_text(row, 21),
        status: get_text(row, 22),
        accepted_by: get_text(row, 23),
        accepted_at_utc: get_ts(row, 24),
        queue_state: get_text(row, 25),
        next_action: get_text(row, 26),
        summary: get_text(row, 27),
        intent_payload: get_json(row, 28),
        intent_payload_hash: get_text(row, 29),
        dispatch_request,
        latest_execution_result,
    })
}

const CONTROL_TASK_COLUMNS: &str = "\
    receipt_id, intent_id, task_id, root_task_id, source_system, \
    channel_id, external_thread_id, user_id, project_id, session_id, \
    input_text, attachments_ref_json, interaction_summary, requested_goal, \
    requested_constraints_json, priority_hint, risk_hint, requested_mode, \
    observed_at_utc, observed_at_local, timezone, node_id, status, \
    accepted_by, accepted_at_utc, queue_state, next_action, summary, \
    intent_payload_json, intent_payload_hash, dispatch_request_json, \
    latest_execution_result_json";

#[async_trait]
impl ControlPlaneStore for LibSqlBackend {
    async fn accept_control_task(
        &self,
        task: &ControlTaskRecord,
        accepted_event: &TaskEventRecord,
    ) -> Result<ControlTaskAcceptance, DatabaseError> {
        let conn = self.connect().await?;
        let attachments_ref_json = encode_json(&task.attachments_ref)?;
        let requested_constraints_json = encode_json(&task.requested_constraints)?;
        let intent_payload_json = encode_json(&task.intent_payload)?;
        let dispatch_request_json = encode_dispatch_request(&task.dispatch_request)?;
        let latest_execution_result_json = encode_execution_result(&task.latest_execution_result)?;
        let payload_json = encode_json(&accepted_event.payload)?;

        conn.execute("BEGIN IMMEDIATE", params![])
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        let result: Result<ControlTaskAcceptance, DatabaseError> = async {
            let mut rows = conn
                .query(
                    &format!(
                        "SELECT {CONTROL_TASK_COLUMNS} FROM control_tasks WHERE intent_id = ?1 LIMIT 1"
                    ),
                    params![task.intent_id.as_str()],
                )
                .await
                .map_err(|e| DatabaseError::Query(e.to_string()))?;
            if let Some(row) = rows
                .next()
                .await
                .map_err(|e| DatabaseError::Query(e.to_string()))?
            {
                let existing = decode_control_task(&row)?;
                return Ok(if existing.intent_payload_hash == task.intent_payload_hash {
                    ControlTaskAcceptance::Existing(existing)
                } else {
                    ControlTaskAcceptance::Conflict(existing)
                });
            }

            conn.execute(
                r#"
                INSERT INTO control_tasks (
                    receipt_id, intent_id, task_id, root_task_id, source_system,
                    channel_id, external_thread_id, user_id, project_id, session_id,
                    input_text, attachments_ref_json, interaction_summary, requested_goal,
                    requested_constraints_json, priority_hint, risk_hint, requested_mode,
                    observed_at_utc, observed_at_local, timezone, node_id, status,
                    accepted_by, accepted_at_utc, queue_state, next_action, summary,
                    intent_payload_json, intent_payload_hash, dispatch_request_json,
                    latest_execution_result_json
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5,
                    ?6, ?7, ?8, ?9, ?10,
                    ?11, ?12, ?13, ?14,
                    ?15, ?16, ?17, ?18,
                    ?19, ?20, ?21, ?22, ?23,
                    ?24, ?25, ?26, ?27, ?28,
                    ?29, ?30, ?31, ?32
                )
                "#,
                params![
                    task.receipt_id.as_str(),
                    task.intent_id.as_str(),
                    task.task_id.as_str(),
                    task.root_task_id.as_str(),
                    task.source_system.as_str(),
                    task.channel_id.as_deref(),
                    task.external_thread_id.as_deref(),
                    task.user_id.as_str(),
                    task.project_id.as_deref(),
                    task.session_id.as_deref(),
                    task.input_text.as_str(),
                    attachments_ref_json,
                    task.interaction_summary.as_deref(),
                    task.requested_goal.as_str(),
                    requested_constraints_json,
                    task.priority_hint.as_deref(),
                    task.risk_hint.as_deref(),
                    task.requested_mode.as_deref(),
                    fmt_ts(&task.observed_at_utc),
                    task.observed_at_local.as_deref(),
                    task.timezone.as_str(),
                    task.node_id.as_deref(),
                    task.status.as_str(),
                    task.accepted_by.as_str(),
                    fmt_ts(&task.accepted_at_utc),
                    task.queue_state.as_str(),
                    task.next_action.as_str(),
                    task.summary.as_str(),
                    intent_payload_json,
                    task.intent_payload_hash.as_str(),
                    dispatch_request_json,
                    latest_execution_result_json,
                ],
            )
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

            conn.execute(
                r#"
                INSERT INTO control_task_events (
                    event_id, parent_event_id, task_id, event_type, producer_type,
                    producer_id, seq, attempt, causation_id, correlation_id,
                    observed_at_utc, observed_at_local, timezone, node_id, payload_json
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5,
                    ?6, ?7, ?8, ?9, ?10,
                    ?11, ?12, ?13, ?14, ?15
                )
                "#,
                params![
                    accepted_event.event_id.as_str(),
                    accepted_event.parent_event_id.as_deref(),
                    accepted_event.task_id.as_str(),
                    accepted_event.event_type.as_str(),
                    accepted_event.producer_type.as_str(),
                    accepted_event.producer_id.as_str(),
                    accepted_event.seq,
                    accepted_event.attempt,
                    accepted_event.causation_id.as_deref(),
                    accepted_event.correlation_id.as_deref(),
                    fmt_ts(&accepted_event.observed_at_utc),
                    accepted_event.observed_at_local.as_deref(),
                    accepted_event.timezone.as_str(),
                    accepted_event.node_id.as_deref(),
                    payload_json,
                ],
            )
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

            Ok(ControlTaskAcceptance::Inserted(task.clone()))
        }
        .await;

        match &result {
            Ok(_) => {
                conn.execute("COMMIT", params![])
                    .await
                    .map_err(|e| DatabaseError::Query(e.to_string()))?;
            }
            Err(_) => {
                let _ = conn.execute("ROLLBACK", params![]).await;
            }
        }

        result
    }

    async fn get_control_task_by_intent_id(
        &self,
        intent_id: &str,
    ) -> Result<Option<ControlTaskRecord>, DatabaseError> {
        let conn = self.connect().await?;
        let mut rows = conn
            .query(
                &format!("SELECT {CONTROL_TASK_COLUMNS} FROM control_tasks WHERE intent_id = ?1"),
                params![intent_id],
            )
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        match rows
            .next()
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?
        {
            Some(row) => Ok(Some(decode_control_task(&row)?)),
            None => Ok(None),
        }
    }

    async fn list_control_task_events(
        &self,
        task_id: &str,
    ) -> Result<Vec<TaskEventRecord>, DatabaseError> {
        let conn = self.connect().await?;
        let mut rows = conn
            .query(
                r#"
                SELECT event_id, parent_event_id, task_id, event_type, producer_type,
                       producer_id, seq, attempt, causation_id, correlation_id,
                       observed_at_utc, observed_at_local, timezone, node_id, payload_json
                FROM control_task_events
                WHERE task_id = ?1
                ORDER BY seq ASC, event_id ASC
                "#,
                params![task_id],
            )
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        let mut events = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| DatabaseError::Query(e.to_string()))?
        {
            events.push(TaskEventRecord {
                event_id: get_text(&row, 0),
                parent_event_id: get_opt_text(&row, 1),
                task_id: get_text(&row, 2),
                event_type: get_text(&row, 3),
                producer_type: get_text(&row, 4),
                producer_id: get_text(&row, 5),
                seq: get_i64(&row, 6),
                attempt: get_i64(&row, 7) as i32,
                causation_id: get_opt_text(&row, 8),
                correlation_id: get_opt_text(&row, 9),
                observed_at_utc: get_ts(&row, 10),
                observed_at_local: get_opt_text(&row, 11),
                timezone: get_text(&row, 12),
                node_id: get_opt_text(&row, 13),
                payload: get_json(&row, 14),
            });
        }
        Ok(events)
    }
}
