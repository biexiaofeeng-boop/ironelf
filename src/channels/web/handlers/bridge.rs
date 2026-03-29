use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::channels::web::auth::AuthenticatedUser;
use crate::channels::web::server::GatewayState;
use crate::runtime_bridge::{
    BridgeResponseError, ExecutionRequest, RuntimeBridgeCancelResponse,
    RuntimeBridgeErrorEnvelope, RuntimeBridgeEventsResponse, RuntimeBridgeHealthResponse,
    RuntimeBridgeSubmitResponse,
};

#[derive(Debug, Deserialize)]
pub struct RuntimeBridgeEventsQuery {
    #[serde(default)]
    pub cursor: Option<usize>,
}

fn error_response(error: BridgeResponseError) -> (StatusCode, Json<RuntimeBridgeErrorEnvelope>) {
    let status = StatusCode::from_u16(error.http_status())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(error.to_envelope()))
}

pub async fn runtime_bridge_health_handler(
    State(state): State<Arc<GatewayState>>,
) -> Json<RuntimeBridgeHealthResponse> {
    Json(
        state
            .runtime_bridge
            .health(state.store.is_some(), state.job_manager.is_some())
            .await,
    )
}

pub async fn runtime_bridge_submit_handler(
    State(state): State<Arc<GatewayState>>,
    AuthenticatedUser(user): AuthenticatedUser,
    Json(request): Json<ExecutionRequest>,
) -> Result<Json<RuntimeBridgeSubmitResponse>, (StatusCode, Json<RuntimeBridgeErrorEnvelope>)> {
    state
        .runtime_bridge
        .submit(
            request,
            &user.user_id,
            state.store.clone(),
            state.job_manager.clone(),
        )
        .await
        .map(Json)
        .map_err(error_response)
}

pub async fn runtime_bridge_events_handler(
    State(state): State<Arc<GatewayState>>,
    AuthenticatedUser(user): AuthenticatedUser,
    Path(execution_id): Path<String>,
    Query(query): Query<RuntimeBridgeEventsQuery>,
) -> Result<Json<RuntimeBridgeEventsResponse>, (StatusCode, Json<RuntimeBridgeErrorEnvelope>)> {
    state
        .runtime_bridge
        .poll_events(
            &execution_id,
            &user.user_id,
            query.cursor.unwrap_or(0),
            state.store.clone(),
            state.job_manager.clone(),
        )
        .await
        .map(Json)
        .map_err(error_response)
}

pub async fn runtime_bridge_cancel_handler(
    State(state): State<Arc<GatewayState>>,
    AuthenticatedUser(user): AuthenticatedUser,
    Path(execution_id): Path<String>,
) -> Result<Json<RuntimeBridgeCancelResponse>, (StatusCode, Json<RuntimeBridgeErrorEnvelope>)> {
    state
        .runtime_bridge
        .cancel(
            &execution_id,
            &user.user_id,
            state.store.clone(),
            state.job_manager.clone(),
        )
        .await
        .map(Json)
        .map_err(error_response)
}
