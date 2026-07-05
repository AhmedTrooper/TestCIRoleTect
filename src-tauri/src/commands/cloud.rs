use crate::s3::{self, S3Config, BackupEntry};
use crate::AppState;
use tauri::State;
use crate::commands::data::{export_all_data_core, import_data_core, AppDataExport};

// Note: `check_data_dirty` is defined here because it naturally fits with cloud logic.
#[tauri::command]
pub async fn check_data_dirty(state: State<'_, AppState>) -> Result<bool, String> {
    let dirty = state.data_dirty.lock().map_err(|e| e.to_string())?;
    Ok(*dirty)
}

async fn get_setting_direct(state: &AppState, key: &str, default: &str) -> String {
    state.with_db(|conn| {
        let val: String = conn.query_row("SELECT value FROM app_settings WHERE key = ?1", [key], |row| row.get(0)).unwrap_or_default();
        Ok(if val.is_empty() { default.to_string() } else { val })
    }).await.unwrap_or_else(|_| default.to_string())
}

async fn get_s3_config(state: &AppState, access_key_id: String, secret_access_key: String) -> S3Config {
    S3Config {
        endpoint_url: get_setting_direct(state, "s3_endpoint_url", "").await,
        bucket_name: get_setting_direct(state, "s3_bucket_name", "").await,
        region: get_setting_direct(state, "s3_region", "us-east-1").await,
        access_key_id,
        secret_access_key,
        force_path_style: get_setting_direct(state, "s3_force_path_style", "true").await == "true",
    }
}

#[tauri::command]
pub async fn test_s3_connection(
    endpoint_url: String,
    bucket_name: String,
    region: String,
    access_key_id: String,
    secret_access_key: String,
    force_path_style: bool,
) -> Result<String, String> {
    let config = S3Config {
        endpoint_url,
        bucket_name: bucket_name.clone(),
        region,
        access_key_id,
        secret_access_key,
        force_path_style,
    };
    let client = s3::build_s3_client(&config).await?;
    s3::test_connection(&client, &config.bucket_name).await?;
    Ok("Connection successful".to_string())
}

#[tauri::command]
pub async fn upload_backup_to_s3(
    state: State<'_, AppState>,
    access_key_id: String,
    secret_access_key: String,
) -> Result<String, String> {
    let config = get_s3_config(&state, access_key_id, secret_access_key).await;
    let client = s3::build_s3_client(&config).await?;
    
    // Get backup data
    let data = export_all_data_core(&state)?;
    let json = serde_json::to_string(&data).map_err(|e| format!("Failed to serialize data: {}", e))?;
    
    let key = s3::upload_backup(&client, &config.bucket_name, &json).await?;
    
    // Track last successful upload
    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let _ = state.with_db(|conn| {
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            ["s3_last_upload", &now_str],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }).await;
    
    // Reset dirty flag
    if let Ok(mut dirty) = state.data_dirty.lock() {
        *dirty = false;
    }
    
    Ok(key)
}

#[tauri::command]
pub async fn list_s3_backups(
    state: State<'_, AppState>,
    access_key_id: String,
    secret_access_key: String,
) -> Result<Vec<BackupEntry>, String> {
    let config = get_s3_config(&state, access_key_id, secret_access_key).await;
    let client = s3::build_s3_client(&config).await?;
    s3::list_backups(&client, &config.bucket_name, 30).await
}

#[tauri::command]
pub async fn restore_from_s3(
    state: State<'_, AppState>,
    access_key_id: String,
    secret_access_key: String,
    key: String,
    mode: String,
) -> Result<(), String> {
    let config = get_s3_config(&state, access_key_id, secret_access_key).await;
    let client = s3::build_s3_client(&config).await?;
    
    let json_str = s3::download_backup(&client, &config.bucket_name, &key).await?;
    let data: AppDataExport = serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse backup JSON: {}", e))?;
    
    import_data_core(&state, data, mode)?;
    
    // Reset dirty flag
    if let Ok(mut dirty) = state.data_dirty.lock() {
        *dirty = false;
    }
    
    Ok(())
}
