use axum::{response::IntoResponse, Json};
use crate::models::meta::HealthResponse;

pub async fn health() -> impl IntoResponse {
    Json(HealthResponse { status: "ok" })
}


