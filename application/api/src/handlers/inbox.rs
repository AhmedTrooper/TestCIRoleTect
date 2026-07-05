use axum::extract::State;
use axum::{Json, http::StatusCode};
use std::sync::Arc;
use crate::AppState;
use crate::models::{InboxJob, ExtensionConfig};
use nanoid::nanoid;

pub async fn get_all_inbox_jobs(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<InboxJob>>, (StatusCode, String)> {
    let conn = state.db.lock().await;
    
    let mut stmt = conn
        .prepare("SELECT id, url, raw_description, status, created_at FROM inbox_jobs ORDER BY created_at DESC")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let job_iter = stmt
        .query_map([], |row| {
            Ok(InboxJob {
                id: row.get(0)?,
                url: row.get(1)?,
                raw_description: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut jobs = Vec::new();
    for job in job_iter {
        jobs.push(job.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?);
    }
    Ok(Json(jobs))
}

pub async fn delete_inbox_job(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let conn = state.db.lock().await;
    
    conn.execute("DELETE FROM inbox_jobs WHERE id = ?1", [&id])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
    Ok(StatusCode::NO_CONTENT)
}

pub async fn mark_inbox_job_processed(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let conn = state.db.lock().await;
    
    conn.execute("UPDATE inbox_jobs SET status = 'Processed' WHERE id = ?1", [&id])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
    Ok(StatusCode::OK)
}

pub async fn get_extension_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ExtensionConfig>, (StatusCode, String)> {
    let conn = state.db.lock().await;
    
    let secret: String = conn.query_row(
        "SELECT value FROM app_settings WHERE key = 'extension_secret'",
        [],
        |row| row.get(0)
    ).unwrap_or_default();

    let port: String = conn.query_row(
        "SELECT value FROM app_settings WHERE key = 'active_server_port'",
        [],
        |row| row.get(0)
    ).unwrap_or_else(|_| "8080".to_string()); // Default to our API port

    Ok(Json(ExtensionConfig { secret, port }))
}

pub async fn reset_extension_secret(
    State(state): State<Arc<AppState>>,
) -> Result<Json<String>, (StatusCode, String)> {
    let new_secret = nanoid!(32);
    let secret_clone = new_secret.clone();
    
    let conn = state.db.lock().await;
    
    conn.execute(
        "UPDATE app_settings SET value = ?1 WHERE key = 'extension_secret'",
        [&secret_clone],
    ).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(new_secret))
}

#[derive(serde::Deserialize)]
pub struct IngestPayload {
    pub url: Option<String>,
    pub raw_description: String,
    pub secret: String,
}

pub async fn ingest_job(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<IngestPayload>,
) -> Result<StatusCode, (StatusCode, String)> {
    let conn = state.db.lock().await;
    
    // 1. Verify Secret
    let secret: String = conn.query_row(
        "SELECT value FROM app_settings WHERE key = 'extension_secret'",
        [],
        |row| row.get(0)
    ).unwrap_or_default();

    if secret != payload.secret {
        return Err((StatusCode::UNAUTHORIZED, "Invalid secret key".to_string()));
    }

    // 2. Save
    let id = nanoid!(10);
    conn.execute(
        "INSERT INTO inbox_jobs (id, url, raw_description, status) VALUES (?1, ?2, ?3, 'Pending')",
        rusqlite::params![&id, &payload.url, &payload.raw_description],
    ).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::CREATED)
}
