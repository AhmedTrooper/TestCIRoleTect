use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize, Debug)]
pub struct InboxJob {
    pub id: String,
    pub url: Option<String>,
    pub raw_description: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct ExtensionConfig {
    pub secret: String,
    pub port: String,
}

#[tauri::command]
pub async fn get_all_inbox_jobs(state: State<'_, AppState>) -> Result<Vec<InboxJob>, String> {
    state.with_db(|conn| {
        let mut stmt = conn
            .prepare("SELECT id, url, raw_description, status, created_at FROM inbox_jobs ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

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
            .map_err(|e| e.to_string())?;

        let mut jobs = Vec::new();
        for job in job_iter {
            jobs.push(job.map_err(|e| e.to_string())?);
        }
        Ok(jobs)
    }).await
}

#[tauri::command]
pub async fn get_inbox_job_by_id(state: State<'_, AppState>, id: String) -> Result<InboxJob, String> {
    state.with_db(move |conn| {
        let mut stmt = conn
            .prepare("SELECT id, url, raw_description, status, created_at FROM inbox_jobs WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let job = stmt.query_row([&id], |row| {
            Ok(InboxJob {
                id: row.get(0)?,
                url: row.get(1)?,
                raw_description: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
            })
        }).map_err(|e| format!("Inbox job not found: {}", e))?;

        Ok(job)
    }).await
}

#[tauri::command]
pub async fn delete_inbox_job(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.with_db(|conn| {
        conn.execute("DELETE FROM inbox_jobs WHERE id = ?1", [&id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }).await?;
    state.mark_dirty();
    Ok(())
}

#[tauri::command]
pub async fn delete_all_inbox_jobs(state: State<'_, AppState>) -> Result<(), String> {
    state.with_db(|conn| {
        conn.execute("DELETE FROM inbox_jobs", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }).await?;
    state.mark_dirty();
    Ok(())
}

#[tauri::command]
pub async fn mark_inbox_job_processed(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.with_db(|conn| {
        conn.execute("UPDATE inbox_jobs SET status = 'Processed' WHERE id = ?1", [&id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }).await?;
    state.mark_dirty();
    Ok(())
}

#[tauri::command]
pub async fn get_extension_config(state: State<'_, AppState>) -> Result<ExtensionConfig, String> {
    state.with_db(|conn| {
        let secret: String = conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'extension_secret'",
            [],
            |row| row.get(0)
        ).unwrap_or_default();

        let port: String = conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'active_server_port'",
            [],
            |row| row.get(0)
        ).unwrap_or_else(|_| "14201".to_string());

        Ok(ExtensionConfig { secret, port })
    }).await
}

#[tauri::command]
pub async fn reset_extension_secret(state: State<'_, AppState>) -> Result<String, String> {
    let new_secret = nanoid::nanoid!(32);
    let secret_clone = new_secret.clone();
    
    state.with_db(move |conn| {
        conn.execute(
            "UPDATE app_settings SET value = ?1 WHERE key = 'extension_secret'",
            [&secret_clone],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }).await?;
    
    state.mark_dirty();
    Ok(new_secret)
}
