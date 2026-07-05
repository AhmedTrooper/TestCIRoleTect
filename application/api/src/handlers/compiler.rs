use axum::{
    extract::State,
    Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;

#[derive(Serialize, Deserialize)]
pub struct CompilerState {
    pub latex_content: String,
}

pub async fn get_compiler_state(
    State(state): State<Arc<AppState>>,
) -> Result<Json<CompilerState>, (StatusCode, String)> {
    let db = state.db.lock().await;

    let latex_content = db.query_row(
        "SELECT latex_content FROM compiler_state WHERE id = 1",
        [],
        |row| row.get(0),
    ).unwrap_or_else(|_| "".to_string());

    Ok(Json(CompilerState { latex_content }))
}

pub async fn save_compiler_state(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CompilerState>,
) -> Result<StatusCode, (StatusCode, String)> {
    let db = state.db.lock().await;

    db.execute(
        "INSERT OR REPLACE INTO compiler_state (id, latex_content, updated_at) VALUES (1, ?1, CURRENT_TIMESTAMP)",
        [&payload.latex_content],
    ).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
