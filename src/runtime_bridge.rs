use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::bootstrap::ironclaw_base_dir;
use crate::db::Database;
use crate::history::{JobEventRecord, SandboxJobRecord};
use crate::orchestrator::auth::CredentialGrant;
use crate::orchestrator::job_manager::{ContainerJobManager, ContainerState, JobMode};

pub const RUNTIME_BRIDGE_SCHEMA_VERSION: &str = "v1";
pub const RUNTIME_BRIDGE_DEFAULT_MAX_CONCURRENT: usize = 4;

const DEFAULT_TIMEOUT_S: u64 = 600;
const MAX_TIMEOUT_S: u64 = 3_600;
const CREATE_JOB_MAX_ATTEMPTS: u32 = 2;
const CREATE_JOB_RETRY_DELAY_MS: u64 = 250;

fn default_schema_version() -> String {
    RUNTIME_BRIDGE_SCHEMA_VERSION.to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecutionContextRef {
    #[serde(rename = "type")]
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecutionRequest {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    pub trace_id: String,
    pub task_id: String,
    pub execution_id: String,
    pub lane: String,
    pub risk_level: String,
    pub objective: String,
    #[serde(default)]
    pub tool_hints: Vec<String>,
    #[serde(default)]
    pub timeout_s: Option<u64>,
    #[serde(default)]
    pub requires_confirmation: bool,
    #[serde(default)]
    pub context_refs: Vec<ExecutionContextRef>,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeBridgeErrorDetail {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeBridgeErrorEnvelope {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    pub error: RuntimeBridgeErrorDetail,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeBridgeHealthResponse {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    pub ok: bool,
    pub status: String,
    pub mode: String,
    pub capabilities: Vec<String>,
    pub supported_tool_hints: Vec<String>,
    pub active_executions: usize,
    pub max_concurrent: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeBridgeSubmitResponse {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    pub accepted: bool,
    pub execution_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionEvent {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    pub trace_id: String,
    pub task_id: String,
    pub execution_id: String,
    pub event_type: String,
    pub status: String,
    pub timestamp: String,
    pub payload: Value,
    pub evidence_refs: Vec<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvidenceDigest {
    pub count: usize,
    pub kinds: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionReceipt {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    pub trace_id: String,
    pub task_id: String,
    pub execution_id: String,
    pub terminal_state: String,
    pub execution_state: String,
    pub summary: String,
    pub evidence_digest: EvidenceDigest,
    pub next_action: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeBridgeEventsResponse {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    pub execution_id: String,
    pub events: Vec<ExecutionEvent>,
    pub next_cursor: usize,
    pub done: bool,
    pub receipt: Option<ExecutionReceipt>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeBridgeCancelResponse {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    pub execution_id: String,
    pub status: String,
    pub done: bool,
    pub receipt: Option<ExecutionReceipt>,
}

#[derive(Debug, Clone)]
pub struct RuntimeBridgeManager {
    max_concurrent: usize,
    records: Arc<RwLock<HashMap<String, ExecutionRecord>>>,
}

#[derive(Debug, Clone)]
struct ExecutionRecord {
    request: ExecutionRequest,
    user_id: String,
    job_id: Option<Uuid>,
    timeout_s: u64,
    deadline_at: Option<DateTime<Utc>>,
    status: BridgeExecutionStatus,
    synthetic_events: Vec<RecordedEvent>,
    next_synthetic_sequence: u64,
    terminal_summary: Option<String>,
}

#[derive(Debug, Clone)]
struct RecordedEvent {
    sequence: u64,
    timestamp: DateTime<Utc>,
    event: ExecutionEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BridgeExecutionStatus {
    Accepted,
    Running,
    WaitingApproval,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    Blocked,
    Degraded,
    NotImplemented,
}

#[derive(Debug, Clone)]
pub struct BridgeResponseError {
    http_status: u16,
    code: String,
    message: String,
    execution_id: Option<String>,
}

#[derive(Debug, Clone)]
struct PreflightDecision {
    status: BridgeExecutionStatus,
    code: String,
    message: String,
}

#[derive(Debug, Clone)]
struct MergedEvent {
    timestamp: DateTime<Utc>,
    source_order: u8,
    order_key: i64,
    event: ExecutionEvent,
}

#[derive(Debug, Clone)]
struct TerminalOutcome {
    status: BridgeExecutionStatus,
    summary: String,
    event_type: &'static str,
    payload: Value,
}

impl BridgeExecutionStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Running => "running",
            Self::WaitingApproval => "waiting_approval",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
            Self::Blocked => "blocked",
            Self::Degraded => "degraded",
            Self::NotImplemented => "not_implemented",
        }
    }

    fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded
                | Self::Failed
                | Self::Cancelled
                | Self::TimedOut
                | Self::Blocked
                | Self::Degraded
                | Self::NotImplemented
        )
    }

    fn terminal_state(self) -> &'static str {
        match self {
            Self::Succeeded => "DONE",
            Self::Failed => "FAILED",
            Self::Cancelled => "CANCELLED",
            Self::TimedOut => "TIMED_OUT",
            Self::Blocked => "BLOCKED",
            Self::Degraded => "DEGRADED",
            Self::NotImplemented => "NOT_IMPLEMENTED",
            Self::Accepted => "ACCEPTED",
            Self::Running => "RUNNING",
            Self::WaitingApproval => "WAITING_APPROVAL",
        }
    }

    fn receipt_state(self) -> &'static str {
        match self {
            Self::Succeeded => "executed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
            Self::Blocked => "blocked",
            Self::Degraded => "degraded",
            Self::NotImplemented => "not_implemented",
            Self::Accepted => "accepted",
            Self::Running => "running",
            Self::WaitingApproval => "waiting_approval",
        }
    }
}

impl BridgeResponseError {
    fn new(http_status: u16, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            http_status,
            code: code.into(),
            message: message.into(),
            execution_id: None,
        }
    }

    fn with_execution_id(mut self, execution_id: impl Into<String>) -> Self {
        self.execution_id = Some(execution_id.into());
        self
    }

    pub fn to_envelope(&self) -> RuntimeBridgeErrorEnvelope {
        RuntimeBridgeErrorEnvelope {
            schema_version: default_schema_version(),
            error: RuntimeBridgeErrorDetail {
                code: self.code.clone(),
                message: self.message.clone(),
            },
            execution_id: self.execution_id.clone(),
        }
    }

    pub fn http_status(&self) -> u16 {
        self.http_status
    }
}

impl ExecutionRecord {
    fn new(request: ExecutionRequest, user_id: String, timeout_s: u64) -> Self {
        Self {
            request,
            user_id,
            job_id: None,
            timeout_s,
            deadline_at: Some(Utc::now() + chrono::Duration::seconds(timeout_s as i64)),
            status: BridgeExecutionStatus::Accepted,
            synthetic_events: Vec::new(),
            next_synthetic_sequence: 0,
            terminal_summary: None,
        }
    }

    fn add_event(
        &mut self,
        timestamp: DateTime<Utc>,
        event_type: &str,
        status: BridgeExecutionStatus,
        payload: Value,
    ) {
        let sequence = self.next_synthetic_sequence;
        self.next_synthetic_sequence += 1;
        self.synthetic_events.push(RecordedEvent {
            sequence,
            timestamp,
            event: ExecutionEvent {
                schema_version: default_schema_version(),
                trace_id: self.request.trace_id.clone(),
                task_id: self.request.task_id.clone(),
                execution_id: self.request.execution_id.clone(),
                event_type: event_type.to_string(),
                status: status.as_str().to_string(),
                timestamp: timestamp.to_rfc3339(),
                payload,
                evidence_refs: Vec::new(),
            },
        });
    }
}

impl Default for RuntimeBridgeManager {
    fn default() -> Self {
        Self::new(RUNTIME_BRIDGE_DEFAULT_MAX_CONCURRENT)
    }
}

impl RuntimeBridgeManager {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            max_concurrent,
            records: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn health(
        &self,
        store_available: bool,
        job_manager_available: bool,
    ) -> RuntimeBridgeHealthResponse {
        let records = self.records.read().await;
        let active_executions = records
            .values()
            .filter(|record| !record.status.is_terminal())
            .count();
        let ok = store_available && job_manager_available;

        RuntimeBridgeHealthResponse {
            schema_version: default_schema_version(),
            ok,
            status: if ok {
                "ready".to_string()
            } else {
                "degraded".to_string()
            },
            mode: if ok {
                "ok".to_string()
            } else {
                "degraded".to_string()
            },
            capabilities: vec![
                "health".to_string(),
                "submit".to_string(),
                "events".to_string(),
                "cancel".to_string(),
            ],
            supported_tool_hints: supported_tool_hints(),
            active_executions,
            max_concurrent: self.max_concurrent,
        }
    }

    pub async fn submit(
        self: &Arc<Self>,
        request: ExecutionRequest,
        user_id: &str,
        store: Option<Arc<dyn Database>>,
        job_manager: Option<Arc<ContainerJobManager>>,
    ) -> Result<RuntimeBridgeSubmitResponse, BridgeResponseError> {
        validate_request(&request)?;

        let timeout_s = normalize_timeout(request.timeout_s);

        if let Some(existing) = self
            .records
            .read()
            .await
            .get(&request.execution_id)
            .cloned()
        {
            if existing.user_id != user_id {
                return Err(BridgeResponseError::new(
                    409,
                    "runtime_execution_conflict",
                    "Execution ID already belongs to another user",
                ));
            }
            if existing.request.trace_id != request.trace_id
                || existing.request.task_id != request.task_id
            {
                return Err(BridgeResponseError::new(
                    409,
                    "runtime_execution_conflict",
                    "Execution ID already exists with different trace or task identifiers",
                )
                .with_execution_id(request.execution_id));
            }
            return Ok(RuntimeBridgeSubmitResponse {
                schema_version: default_schema_version(),
                accepted: true,
                execution_id: existing.request.execution_id,
                status: existing.status.as_str().to_string(),
                reason: existing.terminal_summary,
            });
        }

        if let Some(decision) = self
            .preflight_decision(&request, store.is_some(), job_manager.is_some())
            .await
        {
            let summary = decision.message.clone();
            let execution_id = request.execution_id.clone();
            self.insert_terminal_record(
                request.clone(),
                user_id,
                timeout_s,
                decision,
                summary.clone(),
            )
            .await;
            let status = self
                .records
                .read()
                .await
                .get(&execution_id)
                .map(|record| record.status.as_str().to_string())
                .unwrap_or_else(|| BridgeExecutionStatus::Blocked.as_str().to_string());
            return Ok(RuntimeBridgeSubmitResponse {
                schema_version: default_schema_version(),
                accepted: true,
                execution_id,
                status,
                reason: Some(summary),
            });
        }

        let Some(store) = store else {
            return Err(BridgeResponseError::new(
                503,
                "runtime_store_unavailable",
                "Runtime bridge store is not available",
            ));
        };
        let Some(job_manager) = job_manager else {
            return Err(BridgeResponseError::new(
                503,
                "runtime_job_manager_unavailable",
                "Runtime bridge job manager is not available",
            ));
        };

        let job_id = Uuid::new_v4();
        let project_dir = create_project_dir(job_id).map_err(|message| {
            BridgeResponseError::new(500, "runtime_project_dir_failed", message)
                .with_execution_id(request.execution_id.clone())
        })?;
        let mode = select_job_mode(&request.tool_hints);
        let now = Utc::now();

        tracing::info!(
            execution_id = %request.execution_id,
            job_id = %job_id,
            job_mode = %mode,
            tool_hints = ?request.tool_hints,
            timeout_s,
            "Runtime bridge admitted execution"
        );

        let sandbox_record = SandboxJobRecord {
            id: job_id,
            task: build_job_task(&request),
            status: "creating".to_string(),
            user_id: user_id.to_string(),
            project_dir: project_dir.display().to_string(),
            success: None,
            failure_reason: None,
            created_at: now,
            started_at: None,
            completed_at: None,
            credential_grants_json: serialize_credential_grants(Vec::<CredentialGrant>::new()),
        };

        store.save_sandbox_job(&sandbox_record).await.map_err(|e| {
            BridgeResponseError::new(
                500,
                "runtime_submit_failed",
                format!("Failed to persist sandbox job: {e}"),
            )
            .with_execution_id(request.execution_id.clone())
        })?;

        if let Err(e) = store.update_sandbox_job_mode(job_id, mode.as_str()).await {
            tracing::warn!(job_id = %job_id, error = %e, "Failed to persist runtime bridge job mode");
        }

        {
            let mut records = self.records.write().await;
            let mut record = ExecutionRecord::new(request.clone(), user_id.to_string(), timeout_s);
            record.job_id = Some(job_id);
            record.status = BridgeExecutionStatus::Accepted;
            record.add_event(
                now,
                "admission",
                BridgeExecutionStatus::Accepted,
                json!({
                    "message": "Runtime execution admitted",
                    "job_id": job_id.to_string(),
                    "job_mode": mode.as_str(),
                    "timeout_s": timeout_s,
                }),
            );
            records.insert(request.execution_id.clone(), record);
        }

        let create_result = self
            .launch_job_with_retry(
                &request.execution_id,
                &sandbox_record.task,
                project_dir.clone(),
                mode,
                Arc::clone(&job_manager),
            )
            .await;

        if let Err(error_message) = create_result {
            let completed_at = Utc::now();
            if let Err(e) = store
                .update_sandbox_job_status(
                    job_id,
                    "failed",
                    Some(false),
                    Some(&error_message),
                    Some(now),
                    Some(completed_at),
                )
                .await
            {
                tracing::warn!(job_id = %job_id, error = %e, "Failed to mark runtime bridge job failed after launch error");
            }
            self.finalize_execution(
                &request.execution_id,
                BridgeExecutionStatus::Failed,
                error_message.clone(),
                "result",
                json!({
                    "message": error_message,
                    "job_id": job_id.to_string(),
                    "job_mode": mode.as_str(),
                    "retry_count": CREATE_JOB_MAX_ATTEMPTS.saturating_sub(1),
                }),
            )
            .await?;

            return Err(BridgeResponseError::new(
                500,
                "runtime_submit_failed",
                "Runtime bridge failed to launch execution",
            )
            .with_execution_id(request.execution_id));
        }

        store
            .update_sandbox_job_status(job_id, "running", None, None, Some(now), None)
            .await
            .map_err(|e| {
                BridgeResponseError::new(
                    500,
                    "runtime_submit_failed",
                    format!("Failed to transition sandbox job to running: {e}"),
                )
                .with_execution_id(request.execution_id.clone())
            })?;

        {
            let mut records = self.records.write().await;
            if let Some(record) = records.get_mut(&request.execution_id) {
                record.status = BridgeExecutionStatus::Running;
                record.add_event(
                    Utc::now(),
                    "status",
                    BridgeExecutionStatus::Running,
                    json!({
                        "message": "Runtime execution started",
                        "job_id": job_id.to_string(),
                        "job_mode": mode.as_str(),
                    }),
                );
            }
        }

        self.spawn_timeout_watch(
            request.execution_id.clone(),
            Arc::clone(&store),
            Arc::clone(&job_manager),
        );

        Ok(RuntimeBridgeSubmitResponse {
            schema_version: default_schema_version(),
            accepted: true,
            execution_id: request.execution_id,
            status: BridgeExecutionStatus::Running.as_str().to_string(),
            reason: None,
        })
    }

    pub async fn poll_events(
        &self,
        execution_id: &str,
        user_id: &str,
        cursor: usize,
        store: Option<Arc<dyn Database>>,
        job_manager: Option<Arc<ContainerJobManager>>,
    ) -> Result<RuntimeBridgeEventsResponse, BridgeResponseError> {
        self.ensure_owned(execution_id, user_id).await?;
        self.reconcile_execution(execution_id, store.clone(), job_manager.clone())
            .await?;

        let record = self
            .records
            .read()
            .await
            .get(execution_id)
            .cloned()
            .ok_or_else(|| {
                BridgeResponseError::new(
                    404,
                    "runtime_execution_unknown",
                    "Execution ID is not known to the runtime bridge",
                )
                .with_execution_id(execution_id.to_string())
            })?;

        let merged_events = self.load_merged_events(&record, store).await?;
        let receipt = if record.status.is_terminal() {
            Some(build_receipt(&record, &merged_events))
        } else {
            None
        };
        let next_cursor = merged_events.len();
        let events = merged_events
            .into_iter()
            .skip(cursor.min(next_cursor))
            .map(|event| event.event)
            .collect();

        Ok(RuntimeBridgeEventsResponse {
            schema_version: default_schema_version(),
            execution_id: execution_id.to_string(),
            events,
            next_cursor,
            done: record.status.is_terminal(),
            receipt,
        })
    }

    pub async fn cancel(
        &self,
        execution_id: &str,
        user_id: &str,
        store: Option<Arc<dyn Database>>,
        job_manager: Option<Arc<ContainerJobManager>>,
    ) -> Result<RuntimeBridgeCancelResponse, BridgeResponseError> {
        self.ensure_owned(execution_id, user_id).await?;
        self.reconcile_execution(execution_id, store.clone(), job_manager.clone())
            .await?;

        let record = self
            .records
            .read()
            .await
            .get(execution_id)
            .cloned()
            .ok_or_else(|| {
                BridgeResponseError::new(
                    404,
                    "runtime_execution_unknown",
                    "Execution ID is not known to the runtime bridge",
                )
                .with_execution_id(execution_id.to_string())
            })?;

        if record.status.is_terminal() {
            let merged_events = self.load_merged_events(&record, store).await?;
            return Ok(RuntimeBridgeCancelResponse {
                schema_version: default_schema_version(),
                execution_id: execution_id.to_string(),
                status: record.status.as_str().to_string(),
                done: true,
                receipt: Some(build_receipt(&record, &merged_events)),
            });
        }

        let Some(store) = store else {
            return Err(BridgeResponseError::new(
                503,
                "runtime_store_unavailable",
                "Runtime bridge store is not available for cancellation",
            )
            .with_execution_id(execution_id.to_string()));
        };
        let Some(job_manager) = job_manager else {
            return Err(BridgeResponseError::new(
                503,
                "runtime_job_manager_unavailable",
                "Runtime bridge job manager is not available for cancellation",
            )
            .with_execution_id(execution_id.to_string()));
        };

        let Some(job_id) = record.job_id else {
            return Err(BridgeResponseError::new(
                409,
                "runtime_cancel_unavailable",
                "Execution has no bound runtime job",
            )
            .with_execution_id(execution_id.to_string()));
        };

        if let Err(e) = job_manager.stop_job(job_id).await {
            tracing::warn!(job_id = %job_id, error = %e, "Runtime bridge cancel failed to stop container cleanly");
        }

        store
            .update_sandbox_job_status(
                job_id,
                "interrupted",
                Some(false),
                Some("Cancelled by runtime bridge"),
                None,
                Some(Utc::now()),
            )
            .await
            .map_err(|e| {
                BridgeResponseError::new(
                    500,
                    "runtime_cancel_failed",
                    format!("Failed to persist cancelled state: {e}"),
                )
                .with_execution_id(execution_id.to_string())
            })?;

        self.finalize_execution(
            execution_id,
            BridgeExecutionStatus::Cancelled,
            "Execution cancelled by runtime bridge".to_string(),
            "cancel",
            json!({
                "message": "Execution cancelled",
                "job_id": job_id.to_string(),
            }),
        )
        .await?;

        let record = self
            .records
            .read()
            .await
            .get(execution_id)
            .cloned()
            .ok_or_else(|| {
                BridgeResponseError::new(
                    404,
                    "runtime_execution_unknown",
                    "Execution ID is not known to the runtime bridge",
                )
                .with_execution_id(execution_id.to_string())
            })?;
        let merged_events = self.load_merged_events(&record, Some(store)).await?;

        Ok(RuntimeBridgeCancelResponse {
            schema_version: default_schema_version(),
            execution_id: execution_id.to_string(),
            status: record.status.as_str().to_string(),
            done: true,
            receipt: Some(build_receipt(&record, &merged_events)),
        })
    }

    async fn preflight_decision(
        &self,
        request: &ExecutionRequest,
        store_available: bool,
        job_manager_available: bool,
    ) -> Option<PreflightDecision> {
        if request.lane != "runtime" {
            return Some(PreflightDecision {
                status: BridgeExecutionStatus::Blocked,
                code: "runtime_lane_blocked".to_string(),
                message: "Only lane=runtime is accepted by the runtime bridge".to_string(),
            });
        }

        if request.requires_confirmation {
            return Some(PreflightDecision {
                status: BridgeExecutionStatus::Blocked,
                code: "runtime_confirmation_required".to_string(),
                message: "Execution requires confirmation and cannot start in phase 1".to_string(),
            });
        }

        if !store_available || !job_manager_available {
            return Some(PreflightDecision {
                status: BridgeExecutionStatus::Degraded,
                code: "runtime_unavailable".to_string(),
                message: "Runtime bridge is not ready because sandbox dependencies are unavailable"
                    .to_string(),
            });
        }

        if request
            .tool_hints
            .iter()
            .any(|hint| hint == "subagent" || hint == "multi_agent")
        {
            return Some(PreflightDecision {
                status: BridgeExecutionStatus::NotImplemented,
                code: "runtime_capability_not_implemented".to_string(),
                message: "Subagent-style runtime execution is not implemented in phase 1"
                    .to_string(),
            });
        }

        if request.risk_level.eq_ignore_ascii_case("low") && request.tool_hints.is_empty() {
            return Some(PreflightDecision {
                status: BridgeExecutionStatus::Blocked,
                code: "runtime_risk_gate_blocked".to_string(),
                message: "Phase 1 keeps low-risk fast-lane work outside the runtime bridge"
                    .to_string(),
            });
        }

        let unsupported_hints: Vec<String> = request
            .tool_hints
            .iter()
            .filter(|hint| !is_supported_tool_hint(hint))
            .cloned()
            .collect();
        if !unsupported_hints.is_empty() {
            return Some(PreflightDecision {
                status: BridgeExecutionStatus::Blocked,
                code: "runtime_capability_missing".to_string(),
                message: format!(
                    "Unsupported tool hints for runtime bridge: {}",
                    unsupported_hints.join(", ")
                ),
            });
        }

        let active = self
            .records
            .read()
            .await
            .values()
            .filter(|record| !record.status.is_terminal())
            .count();
        if active >= self.max_concurrent {
            return Some(PreflightDecision {
                status: BridgeExecutionStatus::Blocked,
                code: "runtime_capacity_exceeded".to_string(),
                message: format!(
                    "Runtime bridge reached its max concurrent limit ({})",
                    self.max_concurrent
                ),
            });
        }

        None
    }

    async fn insert_terminal_record(
        &self,
        request: ExecutionRequest,
        user_id: &str,
        timeout_s: u64,
        decision: PreflightDecision,
        summary: String,
    ) {
        let mut record = ExecutionRecord::new(request.clone(), user_id.to_string(), timeout_s);
        record.status = decision.status;
        record.terminal_summary = Some(summary.clone());
        let payload = json!({
            "message": summary,
            "code": decision.code,
        });
        record.add_event(Utc::now(), "admission", decision.status, payload);
        self.records
            .write()
            .await
            .insert(request.execution_id.clone(), record);
    }

    async fn ensure_owned(
        &self,
        execution_id: &str,
        user_id: &str,
    ) -> Result<(), BridgeResponseError> {
        let records = self.records.read().await;
        let Some(record) = records.get(execution_id) else {
            return Err(BridgeResponseError::new(
                404,
                "runtime_execution_unknown",
                "Execution ID is not known to the runtime bridge",
            )
            .with_execution_id(execution_id.to_string()));
        };
        if record.user_id != user_id {
            return Err(BridgeResponseError::new(
                404,
                "runtime_execution_unknown",
                "Execution ID is not known to the runtime bridge",
            )
            .with_execution_id(execution_id.to_string()));
        }
        Ok(())
    }

    async fn launch_job_with_retry(
        &self,
        execution_id: &str,
        task: &str,
        project_dir: PathBuf,
        mode: JobMode,
        job_manager: Arc<ContainerJobManager>,
    ) -> Result<(), String> {
        let job_id = {
            let records = self.records.read().await;
            records
                .get(execution_id)
                .and_then(|record| record.job_id)
                .ok_or_else(|| "Execution lost its job binding before launch".to_string())?
        };

        let mut last_error = None;
        for attempt in 1..=CREATE_JOB_MAX_ATTEMPTS {
            match job_manager
                .create_job(
                    job_id,
                    task,
                    Some(project_dir.clone()),
                    mode,
                    Some(execution_id.to_string()),
                    Vec::<CredentialGrant>::new(),
                )
                .await
            {
                Ok(_) => {
                    tracing::info!(
                        execution_id,
                        job_id = %job_id,
                        job_mode = %mode,
                        attempt,
                        "Runtime bridge launched container"
                    );
                    return Ok(());
                }
                Err(e) => {
                    let message = e.to_string();
                    last_error = Some(message.clone());
                    tracing::warn!(
                        execution_id,
                        job_id = %job_id,
                        job_mode = %mode,
                        attempt,
                        error = %message,
                        "Runtime bridge failed to launch container"
                    );
                    if attempt < CREATE_JOB_MAX_ATTEMPTS {
                        let mut records = self.records.write().await;
                        if let Some(record) = records.get_mut(execution_id) {
                            record.add_event(
                                Utc::now(),
                                "retry",
                                BridgeExecutionStatus::Running,
                                json!({
                                    "message": "Runtime launch failed, retrying",
                                    "attempt": attempt,
                                    "max_attempts": CREATE_JOB_MAX_ATTEMPTS,
                                    "reason": message,
                                }),
                            );
                        }
                        drop(records);
                        tokio::time::sleep(Duration::from_millis(CREATE_JOB_RETRY_DELAY_MS)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| "Runtime launch failed".to_string()))
    }

    fn spawn_timeout_watch(
        self: &Arc<Self>,
        execution_id: String,
        store: Arc<dyn Database>,
        job_manager: Arc<ContainerJobManager>,
    ) {
        let manager = Arc::clone(self);
        tokio::spawn(async move {
            let timeout_s = {
                let records = manager.records.read().await;
                records
                    .get(&execution_id)
                    .map(|record| record.timeout_s)
                    .unwrap_or(DEFAULT_TIMEOUT_S)
            };
            tokio::time::sleep(Duration::from_secs(timeout_s)).await;
            if let Err(e) = manager
                .timeout_execution(&execution_id, store, job_manager)
                .await
            {
                tracing::debug!(execution_id, error = %e.message, "Runtime bridge timeout watcher exited without transition");
            }
        });
    }

    async fn timeout_execution(
        &self,
        execution_id: &str,
        store: Arc<dyn Database>,
        job_manager: Arc<ContainerJobManager>,
    ) -> Result<(), BridgeResponseError> {
        let record = self
            .records
            .read()
            .await
            .get(execution_id)
            .cloned()
            .ok_or_else(|| {
                BridgeResponseError::new(
                    404,
                    "runtime_execution_unknown",
                    "Execution ID is not known to the runtime bridge",
                )
                .with_execution_id(execution_id.to_string())
            })?;
        if record.status.is_terminal() {
            return Ok(());
        }
        if let Some(deadline_at) = record.deadline_at
            && Utc::now() < deadline_at
        {
            return Ok(());
        }

        let job_mode = inferred_job_mode(&record, None, None);
        if let Some(job_id) = record.job_id {
            tracing::warn!(
                execution_id,
                job_id = %job_id,
                job_mode,
                timeout_s = record.timeout_s,
                "Runtime bridge timeout watcher is stopping execution"
            );
            if let Err(e) = job_manager.stop_job(job_id).await {
                tracing::warn!(job_id = %job_id, error = %e, "Runtime bridge timeout failed to stop container cleanly");
            }
            if let Err(e) = store
                .update_sandbox_job_status(
                    job_id,
                    "failed",
                    Some(false),
                    Some("Timed out by runtime bridge"),
                    None,
                    Some(Utc::now()),
                )
                .await
            {
                tracing::warn!(job_id = %job_id, error = %e, "Runtime bridge timeout failed to persist terminal state");
            }
        }

        self.finalize_execution(
            execution_id,
            BridgeExecutionStatus::TimedOut,
            format!("Execution timed out after {}s", record.timeout_s),
            "timeout",
            json!({
                "message": format!("Execution exceeded timeout of {}s", record.timeout_s),
                "timeout_s": record.timeout_s,
                "job_id": record.job_id.map(|id| id.to_string()),
                "job_mode": job_mode,
            }),
        )
        .await
    }

    async fn reconcile_execution(
        &self,
        execution_id: &str,
        store: Option<Arc<dyn Database>>,
        job_manager: Option<Arc<ContainerJobManager>>,
    ) -> Result<(), BridgeResponseError> {
        let record = self
            .records
            .read()
            .await
            .get(execution_id)
            .cloned()
            .ok_or_else(|| {
                BridgeResponseError::new(
                    404,
                    "runtime_execution_unknown",
                    "Execution ID is not known to the runtime bridge",
                )
                .with_execution_id(execution_id.to_string())
            })?;

        if record.status.is_terminal() {
            return Ok(());
        }

        let Some(job_id) = record.job_id else {
            return Ok(());
        };

        let handle = if let Some(job_manager) = job_manager {
            job_manager.get_handle(job_id).await
        } else {
            None
        };

        let sandbox_job = if let Some(store) = store.clone() {
            store.get_sandbox_job(job_id).await.map_err(|e| {
                BridgeResponseError::new(
                    500,
                    "runtime_events_failed",
                    format!("Failed to load sandbox job state: {e}"),
                )
                .with_execution_id(execution_id.to_string())
            })?
        } else {
            None
        };

        let recent_job_events = if let Some(store) = store.clone() {
            Some(store.list_job_events(job_id, Some(20)).await.map_err(|e| {
                BridgeResponseError::new(
                    500,
                    "runtime_events_failed",
                    format!("Failed to load runtime event history: {e}"),
                )
                .with_execution_id(execution_id.to_string())
            })?)
        } else {
            None
        };

        let job_mode = inferred_job_mode(&record, handle.as_ref().map(|h| h.mode), None);

        tracing::debug!(
            execution_id,
            job_id = %job_id,
            job_mode,
            handle_present = handle.is_some(),
            sandbox_status = sandbox_job.as_ref().map(|job| job.status.as_str()).unwrap_or("missing"),
            completion_reported = handle
                .as_ref()
                .and_then(|h| h.completion_result.as_ref())
                .is_some(),
            recent_event_count = recent_job_events.as_ref().map(Vec::len).unwrap_or(0),
            "Runtime bridge reconciling execution"
        );

        if let Some(handle) = handle.as_ref()
            && let Some(result) = handle.completion_result.clone()
        {
            let status = if result.success {
                BridgeExecutionStatus::Succeeded
            } else {
                BridgeExecutionStatus::Failed
            };
            let summary = result.message.clone().unwrap_or_else(|| {
                if result.success {
                    "Execution completed successfully".to_string()
                } else {
                    "Execution failed inside the runtime plane".to_string()
                }
            });
            if let Some(store) = store.as_ref() {
                let db_status = if result.success {
                    "completed"
                } else {
                    "failed"
                };
                let db_message = summary.clone();
                if let Err(e) = store
                    .update_sandbox_job_status(
                        job_id,
                        db_status,
                        Some(result.success),
                        Some(&db_message),
                        None,
                        Some(Utc::now()),
                    )
                    .await
                {
                    tracing::warn!(job_id = %job_id, error = %e, "Runtime bridge failed to backfill terminal sandbox status");
                }
            }
            tracing::info!(
                execution_id,
                job_id = %job_id,
                job_mode = handle.mode.as_str(),
                terminal_status = status.as_str(),
                "Runtime bridge finalized from worker completion report"
            );
            self.finalize_execution(
                execution_id,
                status,
                summary.clone(),
                "result",
                json!({
                    "message": summary,
                    "job_id": job_id.to_string(),
                    "job_mode": handle.mode.as_str(),
                }),
            )
            .await?;
            return Ok(());
        }

        if let Some(outcome) = sandbox_job.as_ref().and_then(|job| {
            terminal_outcome_from_sandbox_job(job, record.timeout_s, job_id, &job_mode)
        }) {
            tracing::info!(
                execution_id,
                job_id = %job_id,
                job_mode,
                terminal_status = outcome.status.as_str(),
                "Runtime bridge finalized from persisted sandbox status"
            );
            self.finalize_execution(
                execution_id,
                outcome.status,
                outcome.summary.clone(),
                outcome.event_type,
                outcome.payload,
            )
            .await?;
            return Ok(());
        }

        if let Some(outcome) = recent_job_events
            .as_ref()
            .and_then(|events| terminal_outcome_from_job_events(events, job_id, &job_mode))
        {
            tracing::info!(
                execution_id,
                job_id = %job_id,
                job_mode,
                terminal_status = outcome.status.as_str(),
                "Runtime bridge finalized from persisted runtime result event"
            );
            self.finalize_execution(
                execution_id,
                outcome.status,
                outcome.summary.clone(),
                outcome.event_type,
                outcome.payload,
            )
            .await?;
            return Ok(());
        }

        if let Some(handle) = handle.as_ref()
            && matches!(
                handle.state,
                ContainerState::Stopped | ContainerState::Failed
            )
        {
            let summary = "Runtime container stopped before reporting a final receipt".to_string();
            if let Some(store) = store.as_ref()
                && let Err(e) = store
                    .update_sandbox_job_status(
                        job_id,
                        "failed",
                        Some(false),
                        Some(&summary),
                        None,
                        Some(Utc::now()),
                    )
                    .await
            {
                tracing::warn!(job_id = %job_id, error = %e, "Runtime bridge failed to persist unexpected stop");
            }
            tracing::warn!(
                execution_id,
                job_id = %job_id,
                job_mode = handle.mode.as_str(),
                container_state = %handle.state,
                "Runtime bridge finalized unexpected container stop"
            );
            self.finalize_execution(
                execution_id,
                BridgeExecutionStatus::Failed,
                summary.clone(),
                "result",
                json!({
                    "message": summary,
                    "job_id": job_id.to_string(),
                    "job_mode": handle.mode.as_str(),
                }),
            )
            .await?;
            return Ok(());
        }

        if sandbox_job.is_some() {
            let waiting_approval = self
                .load_merged_events(&record, store.clone())
                .await?
                .iter()
                .rev()
                .find(|event| event.event.event_type == "status")
                .map(|event| event.event.status == BridgeExecutionStatus::WaitingApproval.as_str())
                .unwrap_or(false);

            let mut records = self.records.write().await;
            if let Some(current) = records.get_mut(execution_id)
                && !current.status.is_terminal()
            {
                current.status = if waiting_approval {
                    BridgeExecutionStatus::WaitingApproval
                } else {
                    BridgeExecutionStatus::Running
                };
            }
        }

        Ok(())
    }

    async fn finalize_execution(
        &self,
        execution_id: &str,
        status: BridgeExecutionStatus,
        summary: String,
        event_type: &str,
        payload: Value,
    ) -> Result<(), BridgeResponseError> {
        let mut records = self.records.write().await;
        let record = records.get_mut(execution_id).ok_or_else(|| {
            BridgeResponseError::new(
                404,
                "runtime_execution_unknown",
                "Execution ID is not known to the runtime bridge",
            )
            .with_execution_id(execution_id.to_string())
        })?;
        if record.status.is_terminal() {
            return Ok(());
        }
        record.status = status;
        record.terminal_summary = Some(summary);
        record.add_event(Utc::now(), event_type, status, payload);
        Ok(())
    }

    async fn load_merged_events(
        &self,
        record: &ExecutionRecord,
        store: Option<Arc<dyn Database>>,
    ) -> Result<Vec<MergedEvent>, BridgeResponseError> {
        let mut merged: Vec<MergedEvent> = record
            .synthetic_events
            .iter()
            .map(|event| MergedEvent {
                timestamp: event.timestamp,
                source_order: 0,
                order_key: event.sequence as i64,
                event: event.event.clone(),
            })
            .collect();

        if let Some(job_id) = record.job_id
            && let Some(store) = store
        {
            let job_events = store.list_job_events(job_id, None).await.map_err(|e| {
                BridgeResponseError::new(
                    500,
                    "runtime_events_failed",
                    format!("Failed to load runtime events: {e}"),
                )
                .with_execution_id(record.request.execution_id.clone())
            })?;

            merged.extend(
                job_events
                    .iter()
                    .map(|event| map_job_event(&record.request, record.status, event)),
            );
        }

        merged.sort_by_key(|event| (event.timestamp, event.source_order, event.order_key));
        Ok(merged)
    }
}

fn validate_request(request: &ExecutionRequest) -> Result<(), BridgeResponseError> {
    if request.schema_version != RUNTIME_BRIDGE_SCHEMA_VERSION {
        return Err(BridgeResponseError::new(
            400,
            "runtime_schema_unsupported",
            format!(
                "Unsupported schema_version '{}', expected '{}'",
                request.schema_version, RUNTIME_BRIDGE_SCHEMA_VERSION
            ),
        ));
    }

    for (field_name, value) in [
        ("trace_id", request.trace_id.as_str()),
        ("task_id", request.task_id.as_str()),
        ("execution_id", request.execution_id.as_str()),
        ("lane", request.lane.as_str()),
        ("risk_level", request.risk_level.as_str()),
        ("objective", request.objective.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(BridgeResponseError::new(
                400,
                "runtime_request_invalid",
                format!("Field '{field_name}' must not be empty"),
            ));
        }
    }

    Ok(())
}

fn normalize_timeout(timeout_s: Option<u64>) -> u64 {
    timeout_s
        .unwrap_or(DEFAULT_TIMEOUT_S)
        .clamp(1, MAX_TIMEOUT_S)
}

fn select_job_mode(tool_hints: &[String]) -> JobMode {
    if tool_hints.iter().any(|hint| {
        matches!(
            hint.as_str(),
            "browser" | "mock_browser" | "web_search" | "web_fetch"
        )
    }) {
        JobMode::ClaudeCode
    } else {
        JobMode::Worker
    }
}

fn supported_tool_hints() -> Vec<String> {
    [
        "browser",
        "mock_browser",
        "web_search",
        "web_fetch",
        "http",
        "shell",
        "file",
        "workspace",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn is_supported_tool_hint(hint: &str) -> bool {
    supported_tool_hints().iter().any(|item| item == hint)
}

fn build_job_task(request: &ExecutionRequest) -> String {
    let mut task = format!(
        "Objective: {}\nTrace ID: {}\nTask ID: {}\nExecution ID: {}",
        request.objective, request.trace_id, request.task_id, request.execution_id
    );

    if !request.tool_hints.is_empty() {
        task.push_str(&format!("\nTool hints: {}", request.tool_hints.join(", ")));
    }
    if !request.context_refs.is_empty() {
        let refs = request
            .context_refs
            .iter()
            .map(|reference| format!("{}:{}", reference.kind, reference.id))
            .collect::<Vec<_>>()
            .join(", ");
        task.push_str(&format!("\nContext refs: {refs}"));
    }
    if let Some(instruction) = request.payload.get("instruction").and_then(Value::as_str) {
        task.push_str(&format!("\nInstruction: {instruction}"));
    } else if !request.payload.is_null() && request.payload != Value::Object(Default::default()) {
        task.push_str(&format!("\nPayload: {}", request.payload));
    }
    task.push_str("\nReturn structured evidence and a concise completion summary.");
    task
}

fn create_project_dir(job_id: Uuid) -> Result<PathBuf, String> {
    let base = ironclaw_base_dir().join("projects");
    std::fs::create_dir_all(&base).map_err(|e| {
        format!(
            "failed to create runtime projects base {}: {e}",
            base.display()
        )
    })?;
    let canonical_base = base.canonicalize().map_err(|e| {
        format!(
            "failed to canonicalize runtime projects base {}: {e}",
            base.display()
        )
    })?;
    let project_dir = canonical_base.join(job_id.to_string());
    std::fs::create_dir_all(&project_dir).map_err(|e| {
        format!(
            "failed to create runtime project directory {}: {e}",
            project_dir.display()
        )
    })?;
    project_dir.canonicalize().map_err(|e| {
        format!(
            "failed to canonicalize runtime project directory {}: {e}",
            project_dir.display()
        )
    })
}

fn serialize_credential_grants(grants: Vec<CredentialGrant>) -> String {
    match serde_json::to_string(&grants) {
        Ok(serialized) => serialized,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to serialize runtime bridge credential grants");
            "[]".to_string()
        }
    }
}

fn reason_mentions_cancel(reason: Option<&str>) -> bool {
    reason
        .map(|reason| reason.to_ascii_lowercase().contains("cancel"))
        .unwrap_or(false)
}

fn reason_mentions_timeout(reason: Option<&str>) -> bool {
    reason
        .map(|reason| {
            let lowered = reason.to_ascii_lowercase();
            lowered.contains("timeout") || lowered.contains("timed out")
        })
        .unwrap_or(false)
}

fn map_job_event(
    request: &ExecutionRequest,
    current_status: BridgeExecutionStatus,
    event: &JobEventRecord,
) -> MergedEvent {
    let (event_type, status) = match event.event_type.as_str() {
        "tool_use" => ("tool_started".to_string(), BridgeExecutionStatus::Running),
        "tool_result" => ("tool_result".to_string(), BridgeExecutionStatus::Running),
        "reasoning" => ("reasoning".to_string(), BridgeExecutionStatus::Running),
        "result" => {
            let status = match (
                event.data.get("success").and_then(Value::as_bool),
                event.data.get("status").and_then(Value::as_str),
            ) {
                (Some(true), _) | (_, Some("completed")) => BridgeExecutionStatus::Succeeded,
                (Some(false), _) | (_, Some("error" | "failed")) => BridgeExecutionStatus::Failed,
                _ => current_status,
            };
            ("result".to_string(), status)
        }
        "message" => ("message".to_string(), BridgeExecutionStatus::Running),
        _ => {
            let waiting_approval = event
                .data
                .get("message")
                .and_then(Value::as_str)
                .map(|message| {
                    let lowered = message.to_ascii_lowercase();
                    lowered.contains("approval") || lowered.contains("auth required")
                })
                .unwrap_or(false);
            let status = if waiting_approval {
                BridgeExecutionStatus::WaitingApproval
            } else {
                BridgeExecutionStatus::Running
            };
            ("status".to_string(), status)
        }
    };

    let evidence_refs = collect_evidence_refs(&event.data);
    let payload = event.data.clone();

    MergedEvent {
        timestamp: event.created_at,
        source_order: 1,
        order_key: event.id,
        event: ExecutionEvent {
            schema_version: default_schema_version(),
            trace_id: request.trace_id.clone(),
            task_id: request.task_id.clone(),
            execution_id: request.execution_id.clone(),
            event_type,
            status: status.as_str().to_string(),
            timestamp: event.created_at.to_rfc3339(),
            payload,
            evidence_refs,
        },
    }
}

fn collect_evidence_refs(data: &Value) -> Vec<Value> {
    let mut evidence_refs = Vec::new();
    for key in ["path", "file", "url", "artifact", "snapshot"] {
        if let Some(value) = data.get(key) {
            evidence_refs.push(json!({ key: value.clone() }));
        }
    }
    evidence_refs
}

fn terminal_outcome_from_sandbox_job(
    sandbox_job: &SandboxJobRecord,
    timeout_s: u64,
    job_id: Uuid,
    job_mode: &str,
) -> Option<TerminalOutcome> {
    let status = match sandbox_job.status.as_str() {
        "completed" => BridgeExecutionStatus::Succeeded,
        "failed" => {
            if reason_mentions_timeout(sandbox_job.failure_reason.as_deref()) {
                BridgeExecutionStatus::TimedOut
            } else if reason_mentions_cancel(sandbox_job.failure_reason.as_deref()) {
                BridgeExecutionStatus::Cancelled
            } else {
                BridgeExecutionStatus::Failed
            }
        }
        "interrupted" => {
            if reason_mentions_cancel(sandbox_job.failure_reason.as_deref()) {
                BridgeExecutionStatus::Cancelled
            } else {
                BridgeExecutionStatus::Failed
            }
        }
        _ => return None,
    };

    let summary = sandbox_job
        .failure_reason
        .clone()
        .unwrap_or_else(|| match status {
            BridgeExecutionStatus::Succeeded => "Execution completed successfully".to_string(),
            BridgeExecutionStatus::Cancelled => "Execution was cancelled".to_string(),
            BridgeExecutionStatus::TimedOut => format!("Execution timed out after {timeout_s}s"),
            _ => "Execution ended with a runtime failure".to_string(),
        });

    Some(TerminalOutcome {
        status,
        event_type: match status {
            BridgeExecutionStatus::Cancelled => "cancel",
            BridgeExecutionStatus::TimedOut => "timeout",
            _ => "result",
        },
        payload: json!({
            "message": summary,
            "job_id": job_id.to_string(),
            "job_mode": job_mode,
        }),
        summary,
    })
}

fn terminal_outcome_from_job_events(
    job_events: &[JobEventRecord],
    job_id: Uuid,
    job_mode: &str,
) -> Option<TerminalOutcome> {
    let event = job_events
        .iter()
        .rev()
        .find(|event| event.event_type == "result" && event.data.is_object())?;

    let status = match (
        event.data.get("success").and_then(Value::as_bool),
        event.data.get("status").and_then(Value::as_str),
    ) {
        (Some(true), _) | (_, Some("completed")) => BridgeExecutionStatus::Succeeded,
        (Some(false), _) | (_, Some("error" | "failed")) => BridgeExecutionStatus::Failed,
        _ => return None,
    };

    let summary = event
        .data
        .get("message")
        .and_then(Value::as_str)
        .or_else(|| event.data.get("content").and_then(Value::as_str))
        .map(ToString::to_string)
        .unwrap_or_else(|| match status {
            BridgeExecutionStatus::Succeeded => "Execution completed successfully".to_string(),
            _ => "Execution failed inside the runtime plane".to_string(),
        });

    let mut payload = event.data.clone();
    if let Some(object) = payload.as_object_mut() {
        object
            .entry("job_id".to_string())
            .or_insert_with(|| json!(job_id.to_string()));
        object
            .entry("job_mode".to_string())
            .or_insert_with(|| json!(job_mode));
    }

    Some(TerminalOutcome {
        status,
        summary,
        event_type: "result",
        payload,
    })
}

fn inferred_job_mode(
    record: &ExecutionRecord,
    handle_mode: Option<JobMode>,
    store_mode: Option<&str>,
) -> String {
    handle_mode
        .map(|mode| mode.as_str().to_string())
        .or_else(|| store_mode.map(ToString::to_string))
        .unwrap_or_else(|| {
            select_job_mode(&record.request.tool_hints)
                .as_str()
                .to_string()
        })
}

fn build_receipt(record: &ExecutionRecord, merged_events: &[MergedEvent]) -> ExecutionReceipt {
    let mut kinds = BTreeSet::new();
    for event in merged_events {
        kinds.insert(classify_evidence_kind(&event.event).to_string());
    }

    let summary = record
        .terminal_summary
        .clone()
        .unwrap_or_else(|| match record.status {
            BridgeExecutionStatus::Succeeded => "Execution completed successfully".to_string(),
            BridgeExecutionStatus::Cancelled => "Execution was cancelled".to_string(),
            BridgeExecutionStatus::TimedOut => {
                format!("Execution timed out after {}s", record.timeout_s)
            }
            BridgeExecutionStatus::Blocked => {
                "Execution was blocked by runtime admission checks".to_string()
            }
            BridgeExecutionStatus::Degraded => {
                "Runtime bridge is degraded and could not execute the request".to_string()
            }
            BridgeExecutionStatus::NotImplemented => {
                "Requested runtime capability is not implemented".to_string()
            }
            BridgeExecutionStatus::Failed => {
                "Execution failed inside the runtime plane".to_string()
            }
            BridgeExecutionStatus::Accepted => "Execution has been accepted".to_string(),
            BridgeExecutionStatus::Running => "Execution is still running".to_string(),
            BridgeExecutionStatus::WaitingApproval => {
                "Execution is waiting for approval".to_string()
            }
        });

    ExecutionReceipt {
        schema_version: default_schema_version(),
        trace_id: record.request.trace_id.clone(),
        task_id: record.request.task_id.clone(),
        execution_id: record.request.execution_id.clone(),
        terminal_state: record.status.terminal_state().to_string(),
        execution_state: record.status.receipt_state().to_string(),
        summary,
        evidence_digest: EvidenceDigest {
            count: merged_events.len(),
            kinds: kinds.into_iter().collect(),
        },
        next_action: "return_to_chimera_core".to_string(),
    }
}

fn classify_evidence_kind(event: &ExecutionEvent) -> &'static str {
    match event.event_type.as_str() {
        "tool_started" => "tool_use",
        "tool_result" => "tool_result",
        "message" => "response",
        "result" => "response",
        "reasoning" => "log",
        "cancel" => "log",
        "timeout" => "log",
        _ => "log",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_db() -> (Arc<dyn crate::db::Database>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("failed to create temp dir: {e}"));
        let path = dir.path().join("runtime-bridge-test.db");
        let backend = crate::db::libsql::LibSqlBackend::new_local(&path)
            .await
            .unwrap_or_else(|e| panic!("failed to create test db: {e}"));
        backend
            .run_migrations()
            .await
            .unwrap_or_else(|e| panic!("failed to run test db migrations: {e}"));
        (Arc::new(backend), dir)
    }

    fn sample_request() -> ExecutionRequest {
        ExecutionRequest {
            schema_version: default_schema_version(),
            trace_id: "trace-1".to_string(),
            task_id: "task-1".to_string(),
            execution_id: "exec-1".to_string(),
            lane: "runtime".to_string(),
            risk_level: "high".to_string(),
            objective: "Collect evidence".to_string(),
            tool_hints: vec!["shell".to_string()],
            timeout_s: Some(30),
            requires_confirmation: false,
            context_refs: vec![ExecutionContextRef {
                kind: "taskops".to_string(),
                id: "task-1".to_string(),
            }],
            payload: json!({"instruction":"echo hello"}),
        }
    }

    #[tokio::test]
    async fn health_reports_degraded_when_runtime_deps_missing() {
        let manager = RuntimeBridgeManager::default();
        let response = manager.health(false, false).await;
        assert!(!response.ok);
        assert_eq!(response.status, "degraded");
        assert_eq!(response.mode, "degraded");
    }

    #[tokio::test]
    async fn submit_blocks_fast_lane_requests_with_mock_compatible_shape() {
        let manager = Arc::new(RuntimeBridgeManager::default());
        let mut request = sample_request();
        request.lane = "fast".to_string();

        let response = manager
            .submit(request.clone(), "alice", None, None)
            .await
            .unwrap_or_else(|e| panic!("submit unexpectedly failed: {}", e.message));

        assert!(response.accepted);
        assert_eq!(response.execution_id, request.execution_id.as_str());
        assert_eq!(response.status, "blocked");

        let events = manager
            .poll_events(&request.execution_id, "alice", 0, None, None)
            .await
            .unwrap_or_else(|e| panic!("poll unexpectedly failed: {}", e.message));

        assert!(events.done);
        assert_eq!(
            events
                .receipt
                .as_ref()
                .map(|receipt| receipt.execution_state.as_str()),
            Some("blocked")
        );
    }

    #[tokio::test]
    async fn poll_events_merges_synthetic_and_runtime_events_with_cursor() {
        let manager = RuntimeBridgeManager::default();
        let request = sample_request();
        let (db, _dir) = test_db().await;
        let job_id = Uuid::new_v4();
        let now = Utc::now();

        db.save_sandbox_job(&SandboxJobRecord {
            id: job_id,
            task: "Collect evidence".to_string(),
            status: "running".to_string(),
            user_id: "alice".to_string(),
            project_dir: "/tmp/runtime-bridge".to_string(),
            success: None,
            failure_reason: None,
            created_at: now,
            started_at: Some(now),
            completed_at: None,
            credential_grants_json: "[]".to_string(),
        })
        .await
        .unwrap_or_else(|e| panic!("failed to save sandbox job: {e}"));
        db.save_job_event(
            job_id,
            "tool_use",
            &json!({
                "tool_name": "shell",
                "input": {"command":"echo hello"},
            }),
        )
        .await
        .unwrap_or_else(|e| panic!("failed to save job event: {e}"));
        db.save_job_event(
            job_id,
            "result",
            &json!({
                "status": "completed",
                "content": "done",
            }),
        )
        .await
        .unwrap_or_else(|e| panic!("failed to save result event: {e}"));

        let mut record = ExecutionRecord::new(request.clone(), "alice".to_string(), 30);
        record.job_id = Some(job_id);
        record.status = BridgeExecutionStatus::Running;
        record.add_event(
            now,
            "admission",
            BridgeExecutionStatus::Accepted,
            json!({"message":"admitted"}),
        );
        manager
            .records
            .write()
            .await
            .insert(request.execution_id.clone(), record);

        manager
            .finalize_execution(
                &request.execution_id,
                BridgeExecutionStatus::Succeeded,
                "Execution completed successfully".to_string(),
                "result",
                json!({"message":"done"}),
            )
            .await
            .unwrap_or_else(|e| panic!("failed to finalize execution: {}", e.message));

        let response = manager
            .poll_events(&request.execution_id, "alice", 1, Some(db), None)
            .await
            .unwrap_or_else(|e| panic!("poll unexpectedly failed: {}", e.message));

        assert!(response.done);
        assert!(response.next_cursor >= 3);
        assert_eq!(
            response
                .events
                .first()
                .map(|event| event.event_type.as_str()),
            Some("tool_started")
        );
        assert_eq!(
            response
                .receipt
                .as_ref()
                .map(|receipt| receipt.terminal_state.as_str()),
            Some("DONE")
        );
        assert!(
            response
                .receipt
                .as_ref()
                .map(|receipt| receipt
                    .evidence_digest
                    .kinds
                    .contains(&"response".to_string()))
                .unwrap_or(false)
        );
    }

    #[test]
    fn map_job_event_treats_successful_result_as_succeeded() {
        let request = sample_request();
        let event = JobEventRecord {
            id: 1,
            job_id: Uuid::new_v4(),
            event_type: "result".to_string(),
            data: json!({
                "success": true,
                "message": "done",
            }),
            created_at: Utc::now(),
        };

        let mapped = map_job_event(&request, BridgeExecutionStatus::Running, &event);
        assert_eq!(mapped.event.event_type, "result");
        assert_eq!(
            mapped.event.status,
            BridgeExecutionStatus::Succeeded.as_str()
        );
    }

    #[test]
    fn terminal_outcome_from_job_events_uses_result_event_without_completion_report() {
        let job_id = Uuid::new_v4();
        let events = vec![JobEventRecord {
            id: 7,
            job_id,
            event_type: "result".to_string(),
            data: json!({
                "success": false,
                "message": "Execution failed: tool timed out",
            }),
            created_at: Utc::now(),
        }];

        let outcome = terminal_outcome_from_job_events(&events, job_id, "worker")
            .unwrap_or_else(|| panic!("expected terminal outcome from result event"));

        assert_eq!(outcome.status, BridgeExecutionStatus::Failed);
        assert_eq!(outcome.event_type, "result");
        assert_eq!(outcome.summary, "Execution failed: tool timed out");
        assert_eq!(outcome.payload["job_id"], json!(job_id.to_string()));
        assert_eq!(outcome.payload["job_mode"], json!("worker"));
    }

    #[tokio::test]
    async fn timeout_transitions_running_execution_to_timed_out() {
        let manager = RuntimeBridgeManager::default();
        let request = sample_request();
        let now = Utc::now();

        let mut record = ExecutionRecord::new(request.clone(), "alice".to_string(), 1);
        record.status = BridgeExecutionStatus::Running;
        record.deadline_at = Some(now - chrono::Duration::seconds(1));
        record.add_event(
            now - chrono::Duration::seconds(2),
            "status",
            BridgeExecutionStatus::Running,
            json!({"message":"running"}),
        );
        manager
            .records
            .write()
            .await
            .insert(request.execution_id.clone(), record);

        let (db, _dir) = test_db().await;
        manager
            .timeout_execution(
                &request.execution_id,
                Arc::clone(&db),
                Arc::new(ContainerJobManager::new(
                    crate::orchestrator::job_manager::ContainerJobConfig::default(),
                    crate::orchestrator::auth::TokenStore::new(),
                )),
            )
            .await
            .unwrap_or_else(|e| panic!("timeout unexpectedly failed: {}", e.message));

        let response = manager
            .poll_events(&request.execution_id, "alice", 0, Some(db), None)
            .await
            .unwrap_or_else(|e| panic!("poll unexpectedly failed: {}", e.message));
        assert!(response.done);
        assert_eq!(
            response
                .receipt
                .as_ref()
                .map(|receipt| receipt.execution_state.as_str()),
            Some("timed_out")
        );
    }
}
