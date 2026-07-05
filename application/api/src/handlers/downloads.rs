use axum::extract::State;
use axum::{Json, http::StatusCode};
use std::sync::Arc;
use crate::AppState;
use crate::models::DownloadRecord;
use nanoid::nanoid;

pub async fn get_recent_downloads(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<DownloadRecord>>, (StatusCode, String)> {
    let conn = state.db.lock().await;
    
    let mut stmt = conn
        .prepare("SELECT id, filename, download_type, job_id, content_id, created_at FROM downloads ORDER BY created_at DESC LIMIT 50")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let iter = stmt
        .query_map([], |row| {
            Ok(DownloadRecord {
                id: row.get(0)?,
                filename: row.get(1)?,
                download_type: row.get(2)?,
                job_id: row.get(3)?,
                content_id: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut records = Vec::new();
    for rec in iter {
        records.push(rec.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?);
    }
    Ok(Json(records))
}

pub async fn record_download(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<StatusCode, (StatusCode, String)> {
    let filename = payload["filename"].as_str().ok_or((StatusCode::BAD_REQUEST, "filename missing".to_string()))?;
    let download_type = payload["downloadType"].as_str().ok_or((StatusCode::BAD_REQUEST, "downloadType missing".to_string()))?;
    let job_id = payload["jobId"].as_str();
    let content_id = payload["contentId"].as_str();

    let id = nanoid!(10);
    let conn = state.db.lock().await;
    
    conn.execute(
        "INSERT INTO downloads (id, filename, download_type, job_id, content_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![&id, filename, download_type, job_id, content_id],
    ).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::CREATED)
}
