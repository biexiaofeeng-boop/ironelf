use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};

use crate::channels::web::auth::AuthenticatedUser;
use crate::channels::web::server::GatewayState;
use crate::control_plane::{
    ControlPlaneErrorEnvelope, ControlPlaneManager, ControlPlaneResponseError, TaskIntent,
    TaskReceipt,
};

fn error_response(
    error: ControlPlaneResponseError,
) -> (StatusCode, Json<ControlPlaneErrorEnvelope>) {
    let status =
        StatusCode::from_u16(error.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(error.to_envelope()))
}

pub async fn control_task_accept_handler(
    State(state): State<Arc<GatewayState>>,
    AuthenticatedUser(user): AuthenticatedUser,
    Json(intent): Json<TaskIntent>,
) -> Result<Json<TaskReceipt>, (StatusCode, Json<ControlPlaneErrorEnvelope>)> {
    ControlPlaneManager
        .accept_task_intent(intent, &user.user_id, state.store.clone())
        .await
        .map(Json)
        .map_err(error_response)
}

#[cfg(all(test, feature = "libsql"))]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::Request, middleware, routing::post};
    use tower::ServiceExt;

    use crate::channels::web::auth::{MultiAuthState, auth_middleware};
    use crate::channels::web::server::{ActiveConfigSnapshot, PerUserRateLimiter, RateLimiter};
    use crate::channels::web::sse::SseManager;
    use crate::runtime_bridge::RuntimeBridgeManager;
    use crate::testing::test_db;

    fn build_state(store: Option<Arc<dyn crate::db::Database>>) -> Arc<GatewayState> {
        Arc::new(GatewayState {
            msg_tx: tokio::sync::RwLock::new(None),
            sse: Arc::new(SseManager::new()),
            workspace: None,
            workspace_pool: None,
            session_manager: None,
            log_broadcaster: None,
            log_level_handle: None,
            extension_manager: None,
            tool_registry: None,
            store,
            job_manager: None,
            runtime_bridge: Arc::new(RuntimeBridgeManager::default()),
            prompt_queue: None,
            owner_id: "test".to_string(),
            shutdown_tx: tokio::sync::RwLock::new(None),
            ws_tracker: None,
            llm_provider: None,
            skill_registry: None,
            skill_catalog: None,
            scheduler: None,
            chat_rate_limiter: PerUserRateLimiter::new(30, 60),
            oauth_rate_limiter: RateLimiter::new(10, 60),
            webhook_rate_limiter: RateLimiter::new(10, 60),
            registry_entries: Vec::new(),
            cost_guard: None,
            routine_engine: Arc::new(tokio::sync::RwLock::new(None)),
            startup_time: std::time::Instant::now(),
            active_config: ActiveConfigSnapshot::default(),
            secrets_store: None,
            db_auth: None,
        })
    }

    fn router(state: Arc<GatewayState>) -> Router {
        Router::new()
            .route(
                "/api/control/tasks/accept",
                post(control_task_accept_handler),
            )
            .route(
                "/api/controlplane/task-intents",
                post(control_task_accept_handler),
            )
            .layer(middleware::from_fn_with_state(
                crate::channels::web::auth::CombinedAuthState::from(MultiAuthState::single(
                    "tok-alice".to_string(),
                    "alice".to_string(),
                )),
                auth_middleware,
            ))
            .with_state(state)
    }

    fn sample_intent() -> TaskIntent {
        serde_json::from_value(serde_json::json!({
            "schema_version": "v1",
            "intent_id": "handler-intent-1",
            "source_system": "chimera-core",
            "channel_id": "gateway",
            "external_thread_id": "thread-1",
            "user_id": "alice",
            "project_id": "project-1",
            "session_id": "session-1",
            "input_text": "Check runtime status",
            "attachments_ref": [],
            "interaction_summary": "control-plane handoff",
            "requested_goal": "Return a runtime summary",
            "requested_constraints": {"must_return_json": true},
            "priority_hint": "normal",
            "risk_hint": "low",
            "requested_mode": "worker",
            "observed_at_utc": "2026-04-06T08:00:00Z",
            "observed_at_local": "2026-04-06T16:00:00+08:00",
            "timezone": "Asia/Shanghai",
            "node_id": "chimera-core-a"
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn accept_route_returns_structured_receipt() {
        let (db, _dir) = test_db().await;
        let app = router(build_state(Some(db)));
        let req = Request::builder()
            .method("POST")
            .uri("/api/control/tasks/accept")
            .header("Authorization", "Bearer tok-alice")
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_vec(&sample_intent()).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 8192).await.unwrap();
        let receipt: TaskReceipt = serde_json::from_slice(&body).unwrap();
        assert_eq!(receipt.intent_id, "handler-intent-1");
        assert_eq!(receipt.status, "accepted");
        assert_eq!(receipt.next_action, "dispatch_pending");
    }

    #[tokio::test]
    async fn legacy_accept_route_returns_structured_receipt() {
        let (db, _dir) = test_db().await;
        let app = router(build_state(Some(db)));
        let mut intent = sample_intent();
        intent.intent_id = "handler-intent-legacy-1".to_string();

        let req = Request::builder()
            .method("POST")
            .uri("/api/controlplane/task-intents")
            .header("Authorization", "Bearer tok-alice")
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_vec(&intent).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 8192).await.unwrap();
        let receipt: TaskReceipt = serde_json::from_slice(&body).unwrap();
        assert_eq!(receipt.intent_id, "handler-intent-legacy-1");
        assert_eq!(receipt.status, "accepted");
        assert_eq!(receipt.next_action, "dispatch_pending");
    }
}
