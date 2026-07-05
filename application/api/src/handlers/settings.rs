use axum::extract::State;
use axum::{Json, http::StatusCode};
use std::sync::Arc;
use crate::AppState;
use crate::models::AiConfig;

pub async fn get_ai_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AiConfig>, (StatusCode, String)> {
    let conn = state.db.lock().await;

    let provider: String = conn.query_row(
        "SELECT value FROM app_settings WHERE key = 'ai_provider'",
        [],
        |row| row.get(0)
    ).unwrap_or_else(|_| "openai".to_string());

    let model: String = conn.query_row(
        "SELECT value FROM app_settings WHERE key = 'ai_model'",
        [],
        |row| row.get(0)
    ).unwrap_or_else(|_| "gpt-4o".to_string());

    let has_key: bool = conn.query_row(
        "SELECT COUNT(*) FROM app_settings WHERE key = 'ai_api_key'",
        [],
        |row| row.get::<_, i64>(0)
    ).unwrap_or(0) > 0;

    Ok(Json(AiConfig { provider, model, has_key }))
}

pub async fn save_ai_config(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<StatusCode, (StatusCode, String)> {
    let provider = payload["provider"].as_str().ok_or((StatusCode::BAD_REQUEST, "provider missing".to_string()))?;
    let model = payload["model"].as_str().ok_or((StatusCode::BAD_REQUEST, "model missing".to_string()))?;
    let api_key = payload["apiKey"].as_str();

    let conn = state.db.lock().await;

    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('ai_provider', ?1)",
        [provider],
    ).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('ai_model', ?1)",
        [model],
    ).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Some(key) = api_key {
        if !key.is_empty() {
            conn.execute(
                "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('ai_api_key', ?1)",
                [key],
            ).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
    }

    Ok(StatusCode::OK)
}

pub async fn get_api_key(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<String>, (StatusCode, String)> {
    let _provider = params.get("provider").ok_or((StatusCode::BAD_REQUEST, "provider missing".to_string()))?;
    
    let conn = state.db.lock().await;
    
    let key: String = conn.query_row(
        "SELECT value FROM app_settings WHERE key = 'ai_api_key'",
        [],
        |row| row.get(0)
    ).map_err(|_| (StatusCode::NOT_FOUND, "API key not found".to_string()))?;

    Ok(Json(key))
}
