use crate::AppState;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadRecord {
    pub id: String,
    pub filename: String,
    pub download_type: String,
    pub job_id: Option<String>,
    pub content_id: Option<String>,
    pub created_at: String,
}

#[tauri::command]
pub async fn record_download(
    state: State<'_, AppState>,
    filename: String,
    download_type: String,
    job_id: Option<String>,
    content_id: Option<String>,
) -> Result<String, String> {
    let mut db_guard = state.db.lock().map_err(|e| format!("Mutex error: {}", e))?;
    let conn = db_guard.as_mut().ok_or("Database connection lost")?;

    let id = nanoid!(10);

    conn.execute(
        "INSERT INTO downloads (id, filename, download_type, job_id, content_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        (
            &id,
            &filename,
            &download_type,
            &job_id,
            &content_id,
        ),
    )
    .map_err(|e| e.to_string())?;

    state.mark_dirty();
    Ok(id)
}

#[tauri::command]
pub async fn get_downloads(state: State<'_, AppState>) -> Result<Vec<DownloadRecord>, String> {
    let mut db_guard = state.db.lock().map_err(|e| format!("Mutex error: {}", e))?;
    let conn = db_guard.as_mut().ok_or("Database connection lost")?;

    let mut stmt = conn
        .prepare("SELECT id, filename, download_type, job_id, content_id, created_at FROM downloads ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;

    let downloads = stmt
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
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(downloads)
}
